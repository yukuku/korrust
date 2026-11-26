import consumers.ConsumerKt;

public class Caller {
    public static void main(String[] args) {
        if (ConsumerKt.text() != null || ConsumerKt.qualified() != null) throw new AssertionError();
        if (!new consumers.Assignments().run(new dep.LocalFactory())) throw new AssertionError();
        if (!ConsumerKt.nested().equals("null")) throw new AssertionError();
        if (!ConsumerKt.shadow(new dep.LocalFactory()).equals("local")) throw new AssertionError();
    }
}
