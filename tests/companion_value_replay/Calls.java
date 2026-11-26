package companionvalues.consumer;
public final class Calls {
    public static void main(String[] args) {
        if (ConsumerKt.primary() != 47L) throw new AssertionError("value getter");
        if (!ConsumerKt.name().equals("palette:direct")) throw new AssertionError("reference getter and JVM field");
        if (ConsumerKt.mutate() != 67L) throw new AssertionError("value setter");
        if (!ConsumerKt.nullable()) throw new AssertionError("nullable value getter");
        if (ConsumerKt.version() != 3) throw new AssertionError("constant field");
        if (ConsumerKt.defaultMethod() != 71L) throw new AssertionError("method default metadata");
        System.out.println("companion-values-ok");
    }
}
