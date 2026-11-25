package nov25.moduleconsumer;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.zip.GZIPOutputStream;
import yuku.alkitab.io.BibleReader;
import yuku.alkitab.io.OptionalGzipInputStream;
import yuku.alkitab.io.Utf8Decoder;
import yuku.alkitab.model.Book;
import yuku.alkitab.model.FootnoteEntry;
import yuku.alkitab.model.PericopeBlock;
import yuku.alkitab.model.SingleChapterVerses;
import yuku.alkitab.model.XrefEntry;
import yuku.alkitab.util.IntArrayList;

public final class BridgeCheck {
    private static int checks;
    private static void require(boolean value) {
        if (!value) throw new AssertionError("Downstream check " + checks);
        checks++;
    }
    private static void origin(Class<?> type, String expected) throws Exception {
        Path actual = Path.of(type.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath();
        require(actual.equals(Path.of(expected).toRealPath()));
    }
    private static final class Reader implements BibleReader {
        private final byte[] encoded;
        private final Book book = new Book();
        Reader(byte[] encoded) { this.encoded = encoded; book.shortName = "Demo"; }
        public String getLocale() { return "en"; }
        public String getShortName() { return "demo"; }
        public String getLongName() { return "Demo Bible"; }
        public String getDescription() { return "consumer"; }
        public Book[] loadBooks() { return new Book[] {book}; }
        public SingleChapterVerses loadVerseText(Book book, int chapter, boolean unsplit, boolean lowercased) {
            return chapter == 1 ? ConsumerKt.chapter(encoded, lowercased) : null;
        }
        public int loadPericope(int book, int chapter, IntArrayList aris, List<PericopeBlock> blocks) {
            aris.add(123);
            return blocks.size();
        }
        public XrefEntry getXrefEntry(int arif) { return null; }
        public FootnoteEntry getFootnoteEntry(int arif) { return null; }
    }
    public static void main(String[] args) throws Exception {
        origin(SingleChapterVerses.class, args[0]);
        origin(Utf8Decoder.class, args[1]);
        origin(BibleReader.class, args[1]);
        origin(Book.class, args[0]);
        origin(Class.forName("yuku.alkitab.yes2.Yes2SingleChapterVerses"), args[2]);
        origin(ConsumerKt.class, args[2]);
        ConsumerKt.verify();
        byte[] bytes = "İ ΟΣ 𐐀".getBytes(StandardCharsets.UTF_8);
        BibleReader reader = new Reader(bytes);
        require(reader.getLocale().equals("en") && reader.getShortName().equals("demo"));
        Book book = reader.loadBooks()[0];
        require(book.reference(1, 2).equals("Demo 1:2"));
        SingleChapterVerses chapter = reader.loadVerseText(book, 1, false, false);
        require(chapter.getVerseCount() == 1 && chapter.getVerse(0).equals("İ ΟΣ 𐐀"));
        require(chapter.getVerseNumberText(0).equals("1"));
        require(reader.loadVerseText(book, 1, false, true).getVerse(0).equals("i οσ 𐐨"));
        require(reader.loadVerseText(book, 2, false, false) == null);
        IntArrayList aris = new IntArrayList(0);
        require(reader.loadPericope(0, 1, aris, new ArrayList<>()) == 0 && aris.get(0) == 123);
        require(reader.getXrefEntry(0) == null && reader.getFootnoteEntry(0) == null);
        ByteArrayOutputStream compressed = new ByteArrayOutputStream();
        try (GZIPOutputStream gzip = new GZIPOutputStream(compressed)) { gzip.write(bytes); }
        for (byte[] input : new byte[][] {bytes, compressed.toByteArray()}) {
            try (OptionalGzipInputStream stream = new OptionalGzipInputStream(new ByteArrayInputStream(input))) {
                require(Arrays.equals(stream.readAllBytes(), bytes));
            }
        }
        System.out.println("{\"marker\":\"model_io_downstream\",\"checks\":" + checks + "}");
    }
}
