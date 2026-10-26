package compatibility.calls.lib;
public final class CallEffects {
    private static String trace = "";
    public static void reset() { trace = ""; }
    public static String trace() { return trace; }
    public static int mark(String id, int value) { trace += id; return value; }
    public static boolean condition(String id, boolean value) { trace += id; return value; }
    public static String platform(String id, boolean missing) { trace += id; return missing ? null : "value"; }
    public static int[] ints(String id) { trace += id; return new int[]{2, 3}; }
    public static String[] strings(String id) { trace += id; return new String[]{"a", "b"}; }
    public static int sum(int... values) { int r = 0; for (int x : values) r += x; return r; }
    public static int count(Object... values) { return values.length; }
    public static int choose(int value) { return 100 + value; }
    public static int choose(int... values) { return values.length; }
    public static int mutate(int... values) { int old = values[0]; values[0] = 99; return old; }
    public static String mutateStrings(String... values) { String old = values[0]; values[0] = "changed"; return old; }
    public static int fail() { trace += "X"; throw new IllegalStateException("expected"); }
}
