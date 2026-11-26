import library.Scope;
public class Caller {
    public static void main(String[] args) {
        if (ConsumerKt.classifier() != java.util.List.class ||
            !ConsumerKt.isSet(new Scope(), java.util.Set.class) ||
            ConsumerKt.isSet(new Scope(), java.util.List.class)) throw new AssertionError();
    }
}
