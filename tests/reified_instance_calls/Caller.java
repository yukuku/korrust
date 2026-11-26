import fixture.*;
public class Caller {
    public static void main(String[] args) {
        Scope scope = new Scope("ok");
        if (!ConsumerKt.text(scope, "str") || ConsumerKt.text(scope, 1)) throw new AssertionError();
        if (!"ok!".equals(ConsumerKt.transform(scope, "")) || ConsumerKt.transform(scope, 1) != null) throw new AssertionError();
    }
}
