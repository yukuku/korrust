import matcher.Scope;
public class Caller {
    public static void main(String[] args) {
        Scope scope = new Scope();
        if (!ConsumerKt.defaulted(scope) || ConsumerKt.named(scope) || ConsumerKt.explicit(scope) || scope.getCounter() != 2) throw new AssertionError();
    }
}
