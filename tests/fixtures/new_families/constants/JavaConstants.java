package newfamilies.library;

public final class JavaConstants {
    static { InitializationTrace.initialized = true; }
    public static final int BASE = 7;
    public static final long WIDE = 9007199254740993L;
    public static final String TEXT = "library";
    public static final boolean FLAG = true;
    public static final char SURROGATE = '\uD800';
    public static final float FLOAT = 1.5f;
    public static final double DOUBLE = -0.0;
    public static final int NON_CONSTANT = Integer.parseInt("5");
    private static final int PRIVATE = 3;
    public static int MUTABLE = 4;
}
