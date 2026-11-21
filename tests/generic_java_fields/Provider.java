package genericfields;
public final class Provider {
    public static int calls;
    public static Cell<String> next() { calls++; return new StringCell("next"); }
}
