public final class AppCalls {
    public static void main(String[] args) {
        if (yuku.alkitab.base.util.FormattedVerseText.removeSpecialCodes(null) != null) throw new AssertionError();
        if (!"plain".equals(yuku.alkitab.base.util.FormattedVerseText.removeSpecialCodes("plain"))) throw new AssertionError();
        if (!"AB".equals(yuku.alkitab.base.util.FormattedVerseText.removeSpecialCodes("@0A@9B"))) throw new AssertionError();
        System.out.println("OK");
    }
}
