import android.graphics.Color;
import androidx.core.graphics.ColorUtils;
import yuku.alkitab.base.audio.ui.AudioHighlightColor;

public final class AudioCheck {
    private static int checks;
    private static void require(boolean value) {
        checks++;
        if (!value) throw new AssertionError("audio check=" + checks);
    }
    public static void main(String[] args) {
        int yellow = Color.argb(0x33, 0xff, 0xeb, 0x3b);
        int black = Color.argb(0x33, 0, 0, 0);
        int white = Color.argb(0x33, 0xff, 0xff, 0xff);
        require(AudioHighlightColor.INSTANCE.pickHighlightColor(Color.WHITE, Color.BLACK) == yellow);
        require(AudioHighlightColor.INSTANCE.pickHighlightColor(Color.rgb(0xf4, 0xec, 0xd8), Color.rgb(0x44, 0x33, 0x22)) == yellow);
        for (int background : new int[] {Color.rgb(0xfa, 0xf0, 0x9c), Color.rgb(0xfb, 0xe6, 0x6f), Color.rgb(0xf8, 0xe9, 0x7c)}) {
            int result = AudioHighlightColor.INSTANCE.pickHighlightColor(background, Color.rgb(0x33, 0x33, 0x33));
            require(result == black || result == white);
        }
        for (int[] pair : new int[][] {{Color.WHITE, Color.BLACK}, {Color.rgb(0xf4, 0xec, 0xd8), Color.rgb(0x44, 0x33, 0x22)}, {Color.BLACK, Color.WHITE}, {Color.rgb(0x12, 0x12, 0x12), Color.rgb(0xe6, 0xe6, 0xe6)}, {Color.rgb(0xfa, 0xf0, 0x9c), Color.rgb(0x33, 0x33, 0x33)}}) {
            int result = AudioHighlightColor.INSTANCE.pickHighlightColor(pair[0], pair[1]);
            require(result == yellow || result == black || result == white);
            int composite = ColorUtils.compositeColors(result, pair[0]);
            require(ColorUtils.calculateContrast(pair[1], composite) >= 4.5);
            double[] first = new double[3];
            double[] second = new double[3];
            ColorUtils.colorToLAB(composite, first);
            ColorUtils.colorToLAB(pair[0], second);
            require(ColorUtils.distanceEuclidean(first, second) >= 2.0);
        }
        System.out.println("audio-checks=" + checks);
    }
}
