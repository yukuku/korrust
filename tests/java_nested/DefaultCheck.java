public final class DefaultCheck {
    public static void main(String[] args) {
        if (DefaultAliasKt.originalAvailable() != 64 || DefaultAliasKt.aliasAvailable() != 64) throw new AssertionError("default alias");
        PlainDepot.Bin.value = 95;
        if (DefaultAliasKt.originalAvailable() != 95 || DefaultAliasKt.aliasAvailable() != 95) throw new AssertionError("default mutable alias");
        System.out.println("OK 4");
    }
}
