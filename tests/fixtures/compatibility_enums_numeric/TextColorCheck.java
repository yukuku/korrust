import yuku.alkitab.base.util.TextColorUtil;
import androidx.core.graphics.ColorUtils;

public class TextColorCheck {
    private static void require(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    private static double channel(int value) {
        double normalized = value / 255.0;
        return normalized <= 0.04045 ? normalized / 12.92 : Math.pow((normalized + 0.055) / 1.055, 2.4);
    }

    private static double luminance(int color) {
        return 0.2126 * channel((color >>> 16) & 255) + 0.7152 * channel((color >>> 8) & 255) + 0.0722 * channel(color & 255);
    }

    private static int blendOpaque(int foreground, int background) {
        int alpha = foreground >>> 24;
        int red = (((foreground >>> 16) & 255) * alpha + ((background >>> 16) & 255) * (255 - alpha)) / 255;
        int green = (((foreground >>> 8) & 255) * alpha + ((background >>> 8) & 255) * (255 - alpha)) / 255;
        int blue = ((foreground & 255) * alpha + (background & 255) * (255 - alpha)) / 255;
        return 0xff000000 | (red << 16) | (green << 8) | blue;
    }

    private static double contrast(int foreground, int background) {
        double first = luminance(foreground) + 0.05;
        double second = luminance(background) + 0.05;
        return Math.max(first, second) / Math.min(first, second);
    }

    public static void main(String[] args) {
        require(ColorUtils.calculateLuminance(0xff000000) == 0.0, "actual library black");
        require(ColorUtils.calculateLuminance(0xffffffff) == 1.0, "actual library white");
        require(ColorUtils.calculateContrast(0xff000000, 0xffffffff) == 21.0, "actual library contrast");
        require(TextColorUtil.CHECKED_VERSE_OVERLAY_ALPHA == 160, "constant");
        float[] brightness = {Float.NEGATIVE_INFINITY, -1f, 0f, Math.nextDown(0.5f), 0.5f, Math.nextUp(0.5f), 1f, Float.POSITIVE_INFINITY, Float.NaN};
        for (float value : brightness) require(TextColorUtil.getSearchKeywordByBrightness(value) == (value < 0.5f ? 0xff69f0ae : 0xff00c853), "brightness " + value);
        int observations = 0;
        long digest = 0;
        for (int gray = 0; gray < 256; gray++) {
            for (int alpha : new int[] {0, 1, 128, 255}) {
                int color = (alpha << 24) | (gray << 16) | (gray << 8) | gray;
                int expected = luminance(color) > 0.4 ? 0xff000000 : 0xffffffff;
                int actual = TextColorUtil.getForCheckedVerse(color);
                require(actual == expected, "luminance threshold " + gray + "/" + alpha);
                digest = digest * 31 + Integer.toUnsignedLong(actual);
                observations++;
            }
        }
        int[] palette = {0xff000000, 0xffffffff, 0xff0277bd, 0xfffff59d, 0xffaeaeae, 0xff212121, 0xfff0f0f0, 0x00212121, 0x000000ff};
        for (int font : palette) {
            for (int checked : palette) {
                for (int page : new int[] {0xff000000, 0xfff0f0f0, 0x00f0f0f0}) {
                    for (int alpha : new int[] {0, 1, 127, 128, 160, 254, 255}) {
                        int band = (alpha << 24) | 0x00ffff00;
                        int checkedText = luminance(checked) > 0.4 ? 0xff000000 : 0xffffffff;
                        int overlay = (checked & 0xffffff) | 0xa0000000;
                        int painted = blendOpaque(band, blendOpaque(overlay, page | 0xff000000));
                        int reading = font | 0xff000000;
                        int expected = contrast(checkedText, painted) >= contrast(reading, painted) ? checkedText : reading;
                        int actual = TextColorUtil.getForCheckedVerseHighlight(font, checked, page, band);
                        require(actual == expected, "highlight " + Integer.toHexString(font) + "/" + Integer.toHexString(checked) + "/" + Integer.toHexString(page) + "/" + alpha);
                        digest = digest * 31 + Integer.toUnsignedLong(actual);
                        observations++;
                    }
                }
            }
        }
        System.out.println("text-color-ok:" + observations + ":" + Long.toUnsignedString(digest));
    }
}
