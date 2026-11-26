package nullablevalues.consumer;
public final class PrimitiveOnlyCalls {
    public static void main(String[] args) {
        if (PrimitiveOnlyKt.primitiveOnly() != 11) throw new AssertionError("value class boxing");
        System.out.println("primitive-boxing-ok");
    }
}
