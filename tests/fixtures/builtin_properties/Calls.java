public class Calls {
    public static void main(String[] args) {
        if (properties.ValuesKt.property() != 2) throw new AssertionError("UTF16 property");
        if (properties.ValuesKt.ordinary() != 17) throw new AssertionError("ordinary method");
        if (properties.ValuesKt.extension() != 23) throw new AssertionError("extension resolution");
    }
}
