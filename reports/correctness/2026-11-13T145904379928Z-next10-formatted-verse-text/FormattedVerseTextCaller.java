import yuku.alkitab.base.util.FormattedVerseText;
public class FormattedVerseTextCaller {
    private static int checks;
    private static void expect(String input, boolean force, String expected) {
        String result = FormattedVerseText.removeSpecialCodes(input, force);
        if (!java.util.Objects.equals(result, expected)) throw new AssertionError(input + " => " + result + " != " + expected);
        checks++;
    }
    public static void main(String[] args) {
        expect(null, false, null);
        expect(null, true, null);
        expect("", false, "");
        expect("plain @1x", false, "plain @1x");
        expect("plain @1x", true, "plain x");
        expect("@1Hello@2world", false, "Hello world");
        expect("@1Hello @2world", false, "Hello world");
        expect("@<hidden@>Shown", false, "Shown");
        expect("@<unclosed", false, "unclosed");
        expect("@xHello", false, "Hello");
        expect("@1あ😀@2文", false, "あ😀 文");
        expect("@1A@8B", false, "A B");
        expect("@1A@^B", false, "A B");
        expect("@1A@0@1B", false, "A B");
        expect("@1\t@2X", false, "\tX");
        String plain = new String("plain");
        if (FormattedVerseText.removeSpecialCodes(plain) != plain) throw new AssertionError("identity");
        checks++;
        if (!FormattedVerseText.removeSpecialCodes("@1A@2B").equals("A B")) throw new AssertionError("overload");
        checks++;
        try {
            FormattedVerseText.removeSpecialCodes("@1foo@");
            throw new AssertionError("missing trailing-marker exception");
        } catch (IndexOutOfBoundsException expected) {
            System.out.println(expected.getClass().getName() + ":" + expected.getMessage());
            checks++;
        }
        System.out.println("OK:" + checks);
    }
}
