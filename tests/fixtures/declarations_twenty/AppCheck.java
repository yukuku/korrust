import yuku.alkitab.base.util.BookColorUtil;
public final class AppCheck {
    public static void main(String[] args) {
        int checks = 0;
        for (int value : new int[]{Integer.MIN_VALUE, -1, 0, 38, 39, 65, 66, Integer.MAX_VALUE}) {
            int group = value >= 0 && value <= 38 ? 0 : value >= 39 && value <= 65 ? 1 : 2;
            int[] dark = {0xffff80ab,0xff82b1ff,0xffe0e0e0};
            int[] light = {0xffc2185b,0xff1976d2,0xff424242};
            int[] background = {0xff880e4f,0xff0d47a1,0xff212121};
            if (BookColorUtil.getForegroundOnDark(value) != dark[group] || BookColorUtil.getForegroundOnLight(value) != light[group] || BookColorUtil.getBackground(value) != background[group]) throw new AssertionError(value);
            checks += 3;
        }
        System.out.println("{\"checks\":" + checks + "}");
    }
}
