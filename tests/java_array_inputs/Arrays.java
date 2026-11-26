package dep;

public class Arrays {
    public static int objects(Object[] values) { return values.length; }
    public static int strings(CharSequence[] values) { return values.length; }
    public static int nested(Object[][] values) { return values.length; }
    public static void write(Object[] values) { values[0] = 1; }
}
