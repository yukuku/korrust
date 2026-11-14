package iteration;
public final class Trace {
    public static String log = "";
    public static int record(String event, int result) { log += event; return result; }
}
