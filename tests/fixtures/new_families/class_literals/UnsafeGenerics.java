package newfamilies.dependency;

public final class UnsafeGenerics {
    @SuppressWarnings("unchecked")
    public static <T> T value() { return (T) "erased-runtime"; }
}
