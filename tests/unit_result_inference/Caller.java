public class Caller {
    public static void main(String[] args) throws Exception {
        new Action().perform();
        ConsumerKt.singleton();
        ConsumerKt.explicit();
        if (ConsumerKt.class.getMethod("singleton").getReturnType() != Void.TYPE
                || ConsumerKt.asObject() != kotlin.Unit.INSTANCE) throw new AssertionError();
        OtherUnit other = new OtherUnit();
        if (ConsumerKt.shadowed(other) != other) throw new AssertionError();
    }
}
