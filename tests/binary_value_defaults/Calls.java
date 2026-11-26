package binarydefaults.consumer;
public final class Calls {
    public static void main(String[] args) {
        if (ConsumerKt.topValue() != 54L) throw new AssertionError("static default/explicit values");
        if (ConsumerKt.memberValue() != 45L) throw new AssertionError("member defaults");
        if (ConsumerKt.extensionValue() != 42L) throw new AssertionError("extension receiver mask offset");
        if (ConsumerKt.dispatchValue() != 44L) throw new AssertionError("value dispatch carrier");
        if (!ConsumerKt.referenceValue().equals("reference")) throw new AssertionError("reference carrier");
        if (!ConsumerKt.nullableValue()) throw new AssertionError("nullable carrier");
        if (!ConsumerKt.genericValue().equals("generic")) throw new AssertionError("generic member");
        if (ConsumerKt.wideValue() != 233L) throw new AssertionError("two mask words");
        if (!ConsumerKt.order().equals("ab:a:5")) throw new AssertionError("provided argument order");
        if (!ConsumerKt.defaultOrder().equals("RaK:a:5")) throw new AssertionError("defaults after provided arguments");
        if (ConsumerKt.nullableExtension() != 7L) throw new AssertionError("nullable extension receiver");
        System.out.println("binary-value-defaults-ok");
    }
}
