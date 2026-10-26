import java.util.Arrays;
import java.util.List;
import kotlin.jvm.functions.Function1;
import yuku.alkitab.base.feedback.PrereleaseVersion;
import yuku.alkitab.base.util.LidToAri;
import yuku.alkitab.base.util.ReadingPassage;
import yuku.alkitab.base.util.ReadingRange;
import yuku.alkitab.model.Book;

public final class AppCheck {
    static void eq(Object actual, Object expected) {
        if (!java.util.Objects.equals(actual, expected)) throw new AssertionError(actual + " != " + expected);
    }
    static void prerelease() {
        for (String value : new String[]{"1.0", "1.0-beta", "1.0-beta.x", "1.0-rc.2", "1.0-dev.2-extra", "", "1.0-beta.2\nX"}) {
            eq(PrereleaseVersion.Companion.parse(value), null);
        }
        PrereleaseVersion beta = PrereleaseVersion.Companion.parse("1.0-beta.23");
        eq(beta.getChannel(), "Beta"); eq(beta.getLabel(), "Beta");
        eq(beta.emailSubject("Bible", "1.0-beta.23", "abc"), "Beta feedback Bible 1.0-beta.23 (abc)");
        PrereleaseVersion dev = PrereleaseVersion.Companion.parse("1.0-dev.007");
        eq(dev.getChannel(), "Dev"); eq(dev.getLabel(), "Dev 007");
    }
    static void lid() {
        int[][] samples = {{Integer.MIN_VALUE, 0}, {-1, 0}, {0, 0}, {1, 0x101}, {31, 0x11f}, {32, 0x201}, {56, 0x219}, {57, 0x301}, {1534, 0x10101}, {31082, 0x411601}, {31102, 0x411615}, {31103, 0}, {Integer.MAX_VALUE, 0}};
        for (int[] pair : samples) eq(LidToAri.lidToAri(pair[0]), pair[1]);
        eq(LidToAri.bookIdForLid(0), -1); eq(LidToAri.bookIdForLid(31103), -1);
        eq(LidToAri.bookIdForLid(1), 0); eq(LidToAri.bookIdForLid(1533), 0);
        eq(LidToAri.bookIdForLid(1534), 1); eq(LidToAri.bookIdForLid(31102), 65);
        eq(LidToAri.startLidForBookId(0), 1); eq(LidToAri.endLidForBookId(0), 1533);
        eq(LidToAri.startLidForBookId(65), 30699); eq(LidToAri.endLidForBookId(65), 31102);
        for (int book = 0; book < 66; book++) {
            int start = LidToAri.startLidForBookId(book), end = LidToAri.endLidForBookId(book);
            eq(LidToAri.bookIdForLid(start), book); eq(LidToAri.bookIdForLid(end), book);
            if (book < 65) eq(end + 1, LidToAri.startLidForBookId(book + 1));
        }
    }
    static void reading() {
        Book genesis = new Book(); genesis.bookId = 0; genesis.chapter_count = 2; genesis.verse_counts = new int[]{3, 2};
        StringBuilder calls = new StringBuilder();
        Function1<Integer, Book> lookup = id -> { calls.append(id).append(';'); return id == 0 ? genesis : null; };
        for (int[] values : new int[][]{{}, {0}, {-1, 0}, {0x1000000, 0}, {1, 1}, {0x301, 0x301}, {0x104, 0x104}, {0x102, 0x101}, {0x10101, 0x10101}}) {
            eq(ReadingPassage.resolve(values, lookup), null);
        }
        calls.setLength(0);
        List<ReadingRange> whole = ReadingPassage.resolve(new int[]{0, 0}, lookup);
        eq(calls.toString(), "0;0;");
        eq(whole.size(), 1); eq(whole.get(0).getStart(), 0x101); eq(whole.get(0).getEnd(), 0x202);
        eq(whole.get(0).contains(0x101), true); eq(whole.get(0).contains(0x100), false); eq(whole.get(0).contains(0x203), false);
        List<ReadingRange> chapter = ReadingPassage.resolve(new int[]{0x200, 0x200}, lookup);
        eq(chapter.get(0).getStart(), 0x201); eq(chapter.get(0).getEnd(), 0x202);
        for (String value : new String[]{"", "nonsense", "Genesis 0", "Genesis 3", "Genesis 1:4", "ari:-1", "ari:16777216", "ari:wat", "Genesis 2:2-1:1"}) eq(ReadingPassage.parse(value, lookup), null);
        calls.setLength(0);
        if (!Arrays.equals(ReadingPassage.parse("  Genesis 1:2-3 ", lookup), new int[]{0x102, 0x103})) throw new AssertionError("verse parse");
        eq(calls.toString(), "0;0;");
        if (!Arrays.equals(ReadingPassage.parse("ari:257", lookup), new int[]{0x101, 0x101})) throw new AssertionError("ari parse");
        if (!Arrays.equals(ReadingPassage.parse("Genesis", lookup), new int[]{0, 0})) throw new AssertionError("book parse");
    }
    public static void main(String[] args) {
        switch (args[0]) {
            case "PrereleaseVersion" -> prerelease();
            case "LidToAri" -> lid();
            case "ReadingPassage" -> reading();
            default -> throw new AssertionError(args[0]);
        }
        System.out.println("pass:" + args[0]);
    }
}
