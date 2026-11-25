import genericargs.Defaults;
public class DefaultCalls {
    public static void main(String[] args) {
        Defaults<String> box = new Defaults<>("OK");
        if (box.<String>fallback(null) != null) throw new AssertionError("fallback");
        if (!box.repeat("wrong", "OK").equals("OK")) throw new AssertionError("repeat");
        if (!box.classifier(1, "OK").equals("OK")) throw new AssertionError("classifier");
        if (box.<String>nullable("wrong", null) != null) throw new AssertionError("nullable");
        if (!box.sequence("wrong", "wrong", "OK").equals("OK")) throw new AssertionError("sequence");
        if (!genericargs.DefaultsKt.sourceDefaults().equals("OK")) throw new AssertionError("source defaults");
        System.out.println("OK");
    }
}
