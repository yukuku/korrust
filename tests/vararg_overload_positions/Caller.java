import consumers.ConsumerKt;

public class Caller {
    public static void main(String[] args) {
        Object[] two = {1, 2};
        if (!ConsumerKt.empty().equals("all:0")) throw new AssertionError();
        if (!ConsumerKt.any("p").equals("all:2")) throw new AssertionError();
        if (!ConsumerKt.narrow().equals("p:2")) throw new AssertionError();
        if (!ConsumerKt.spread(two).equals("s:2")) throw new AssertionError();
        if (!ConsumerKt.named(two).equals("n:2")) throw new AssertionError();
        if (!ConsumerKt.numbers().equals("int:2:1")) throw new AssertionError();
        if (!ConsumerKt.formatted().equals("[000000,000001,000002]")) throw new AssertionError();
    }
}
