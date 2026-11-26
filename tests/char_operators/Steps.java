package chars;

public class Steps {
    public static int trace;
    public static char nextChar() { trace = trace * 10 + 1; return 0xffff; }
    public static int nextOffset() { trace = trace * 10 + 2; return 1; }
}
