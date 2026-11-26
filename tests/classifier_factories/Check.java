public final class Check {
    public static void main(String[] args) {
        if (UseKt.imported().value() != 7 || UseKt.alias().value() != 8) throw new AssertionError("interface factory");
        if (!UseKt.inferred().value().equals("generic") || UseKt.explicit().value() != 9) throw new AssertionError("generic factory");
        if (UseKt.constructor().getValue() != 10 || UseKt.factory().getValue() != 4) throw new AssertionError("constructor overload");
        System.out.println("OK");
    }
}
