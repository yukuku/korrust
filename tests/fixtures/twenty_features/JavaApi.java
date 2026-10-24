package twenty.dep;
public final class JavaApi {
    static { Trace.initializations++; }
    public static final int FIXED = 17;
    public static volatile int changing = 5;
    public final int immutable = 23;
    public volatile int mutable = 7;
    private static final JavaApi INSTANCE = new JavaApi();
    public static JavaApi receiver() { Trace.events += "Q"; return INSTANCE; }
    public static JavaApi orderedReceiver() { Trace.events += "R"; return INSTANCE; }
    public static JavaApi nullReceiver() { Trace.events += "R"; return null; }
    public int add2(int a, int b) { Trace.events += "M"; return a + b; }
    public int add(int value) { Trace.events += "M"; return mutable + value; }
    public static int sum(int a, int b) { return a + b; }
    public static String choose(int value) { return "int"; }
    public static String choose(long value) { return "long"; }
    public static String choose(String value) { return "string"; }
    public static String ambiguous(String value) { return "string"; }
    public static String ambiguous(Integer value) { return "integer"; }
    public static int throwing() { Trace.events += "T"; throw new IllegalArgumentException("static"); }
}
