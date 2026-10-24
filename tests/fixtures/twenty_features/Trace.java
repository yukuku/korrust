package twenty.dep;
public final class Trace {
    public static String events = "";
    public static int initializations;
    public static int mark(String text, int value) { events += text; return value; }
    public static long wide(String text, long value) { events += text; return value; }
    public static String text(String text, String value) { events += text; return value; }
}
