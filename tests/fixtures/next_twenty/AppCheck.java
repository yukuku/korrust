import yuku.alkitab.base.widget.VerseNumberPlacementKt;
public final class AppCheck {
    static int checks;
    static void check(String text, int length, boolean expected) {
        checks++;
        if (VerseNumberPlacementKt.startsWithGutterParagraph(text.toCharArray(), length) != expected) throw new AssertionError(text);
    }
    public static void main(String[] args) {
        check("@@@^text", 8, true);
        check("@@@5@1text", 10, true);
        check("@@plain", 7, false);
        check("@@@0text", 8, false);
        check("@@@", 3, false);
        check("@@@5@2", 6, true);
        check("@@@^text", 2, false);
        check("@@@9@7@3\ud83d\ude00", 10, true);
        System.out.println("{\"checks\":" + checks + "}");
    }
}
