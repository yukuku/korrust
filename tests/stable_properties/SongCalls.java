import java.util.*;
import kotlin.jvm.functions.Function1;
import yuku.alkitab.songs.newdoc.*;
public final class SongCalls {
    private static Line plain(String text) { return new Line.Plain(text); }
    private static VerseLine line(String text) { return new VerseLine.Simple(plain(text)); }
    private static PBlock paragraph(String role, String text) { return new PBlock(role, null, null, plain(text)); }
    private static Verse verse(VerseKind kind, VerseLine... lines) { return new Verse(kind, null, Arrays.asList(lines)); }
    private static void eq(Object expected, Object actual) {
        if (!Objects.equals(expected, actual)) throw new AssertionError("expected=" + expected + " actual=" + actual);
    }
    public static void main(String[] args) {
        Locale.setDefault(Locale.forLanguageTag("tr-TR"));
        VerseLine styled = new VerseLine.Wrapped(null, null, new Line.Styled(Arrays.asList(new Span("styled", Collections.emptyList()), new Span("!", Collections.emptyList()))));
        LyricBlock first = new LyricBlock(null, null, null, Arrays.asList(
            verse(VerseKind.NORMAL, line("hello"), styled),
            verse(VerseKind.REFRAIN, line("again")),
            verse(VerseKind.TEXT, line("speak"))));
        LyricBlock second = new LyricBlock(null, null, plain("Alternate"), Arrays.asList(verse(VerseKind.NORMAL, line("other"))));
        SongDocument doc = new SongDocument("7", new Meta("Title", "Original"), Arrays.asList(
            new RowBlock(null, Arrays.asList(paragraph("authors_lyric", "Words"), paragraph("authors_music", "Music"), paragraph("caption", "hidden"))),
            paragraph("tune", "istanbul"), paragraph("musical", "4/4"), first, second,
            new ScriptureBlock(null, null, "hidden"), new YoutubeBlock(null, null, "hidden"), new GapBlock(null)));
        eq(Arrays.asList("7", "Title", "Original", "Words", "Music", "istanbul", "hello", "styled!", "again", "speak", "other"), SongDocumentSearch.searchableTexts(doc));
        eq(Arrays.asList("hello", "styled!", "again", "speak", "other"), SongDocumentSearch.lyricLines(doc));
        final List<Integer> captions = new ArrayList<>();
        Function1<Integer, String> caption = index -> { captions.add(index); return "Version " + index; };
        String rendered = SongDocumentText.render(doc, new StringBuilder("Book"), "John 1", caption, "Refrain");
        eq("Book 7. Title\n(Original)\n\nWords\nMusic\nISTANBUL\n\nJohn 1\n4/4\n\nVersion 1\n 1: hello\n    styled!\n\nRefrain\n    again\n\n    speak\n\n\nAlternate\n 1: other\n\n\n", rendered);
        eq(Arrays.asList(1), captions);
        SongDocument empty = new SongDocument("0", new Meta(null, null), Collections.emptyList());
        eq(Arrays.asList("0"), SongDocumentSearch.searchableTexts(empty));
        eq(Collections.emptyList(), SongDocumentSearch.lyricLines(empty));
        eq("0. \n\n\n\n", SongDocumentText.render(empty, null, null, caption, "R"));
        System.out.println("OK");
    }
}
