package interop.dep;

import java.util.List;
import java.util.function.IntUnaryOperator;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

public final class JavaApi {
    public static String events = "";
    public static int mark(String event, int value) { events += event; return value; }
    public static <T> T identity(T value) { return value; }
    public static <T> T first(List<T> values) { return values.get(0); }
    public static <T extends CharSequence> int bounded(T value) { return value.length(); }
    public static <T> T projected(List<? extends T> values) { return values.get(0); }
    public static String platform(boolean nil) { return nil ? null : "platform"; }
    public static @NotNull String nonnull() { return "annotated"; }
    public static @Nullable String nullable() { return null; }
    public static String choose(Object value) { return "object"; }
    public static String choose(CharSequence value) { return "sequence"; }
    public static String choose(String value) { return "string"; }
    public static int apply(IntUnaryOperator operator, int value) { return operator.applyAsInt(value); }
    public interface ObjectNameOverload {
        boolean equals(String value);
        void run();
    }
    public static int requireNotSam(ObjectNameOverload value) { value.run(); return 1; }
}
