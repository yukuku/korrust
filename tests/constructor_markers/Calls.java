package markers.consumer;
public final class Calls {
    public static void main(String[] args) {
        if (ConsumerKt.numeric() != 83L) throw new AssertionError("carriers");
        if (!ConsumerKt.defaults().equals("default:named:generic:qualified")) throw new AssertionError("defaults");
        if (!ConsumerKt.nullable()) throw new AssertionError("nullable carrier");
        if (ConsumerKt.wide() != 134L) throw new AssertionError("mask words");
        if (!ConsumerKt.order().equals("abS:a")) throw new AssertionError("evaluation order");
        if (!ConsumerKt.evolved().equals("current:59")) throw new AssertionError("hidden overload");
        System.out.println("constructor-markers-ok");
    }
}
