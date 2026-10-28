package newfamilies.dependency;

public final class InitializationProbe {
    private InitializationProbe() {}
    static { InitializationTrace.initializations++; }
    public static InitializationProbe create() {
        InitializationTrace.creations++;
        return new InitializationProbe();
    }
}
