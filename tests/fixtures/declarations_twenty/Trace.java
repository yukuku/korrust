package declarations.support;
public final class Trace {
    public static String events = "";
    public static int mark(int value) { events += value; return value; }
    public static void clear() { events = ""; }
}
