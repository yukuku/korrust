import yuku.alkitab.base.verses.ReadingGuideKt;

public class ReadingGuideColorCheck {
    public static void main(String[] args) {
        if (ReadingGuideKt.readingGuideLineColor(0xff000000, 0xffffffff) != 0xff000000) throw new AssertionError("retain sufficient contrast");
        if (ReadingGuideKt.readingGuideLineColor(0xff222222, 0xff222222) != 0xffffffff) throw new AssertionError("choose white on dark background");
        if (ReadingGuideKt.readingGuideLineColor(0xffcccccc, 0xffeeeeee) != 0xff000000) throw new AssertionError("choose black on light background");
        if (ReadingGuideKt.readingGuideLineColor(0x00000000, 0x00ffffff) != 0xff000000) throw new AssertionError("force opaque channels");
        System.out.println("reading-guide-color-ok");
    }
}
