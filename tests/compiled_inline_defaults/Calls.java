package defaults.consumer;

public class Calls {
    public static void main(String[] args) {
        defaults.library.LibraryKt.setOrder(0);
        if (ConsumerKt.omitted() != 13 || defaults.library.LibraryKt.getOrder() != 12) throw new AssertionError("dependent defaults");
        defaults.library.LibraryKt.setOrder(0);
        if (ConsumerKt.reordered() != 6 || defaults.library.LibraryKt.getOrder() != 51) throw new AssertionError("argument/default order");
        if (ConsumerKt.nonlocal() != 23) throw new AssertionError("non-local return");
        defaults.library.LibraryKt.setOrder(0);
        if (ConsumerKt.finalizer() != 7 || defaults.library.LibraryKt.getOrder() != 29) throw new AssertionError("finally");
        if (!ConsumerKt.reified().equals("java.lang.String")) throw new AssertionError("reified default");
        if (!ConsumerKt.lambdaDefault().equals("kept")) throw new AssertionError("default lambda");
        if (!ConsumerKt.suppliedLambda().equals("2provided")) throw new AssertionError("unused reified default");
        if (ConsumerKt.encoded() != 3) throw new AssertionError("stdlib charset default");
        defaults.library.LibraryKt.setOrder(0);
        if (ConsumerKt.wideMask() != 6 || defaults.library.LibraryKt.getOrder() != 123) throw new AssertionError("two mask words / sign bit");
        if (!ConsumerKt.fieldDefault().equals("field")) throw new AssertionError("class property initializer");
        System.out.println("inline-defaults-ok");
    }
}
