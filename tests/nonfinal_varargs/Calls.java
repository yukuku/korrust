package nonfinal.consumer;

public class Calls {
    public static void main(String[] args) {
        if (ConsumerKt.empty() != 102) throw new AssertionError("empty");
        nonfinal.library.LibraryKt.setOrder(0);
        if (ConsumerKt.filled() != 322 || nonfinal.library.LibraryKt.getOrder() != 456) throw new AssertionError("filled order");
        nonfinal.library.LibraryKt.setOrder(0);
        if (ConsumerKt.reordered() != 117 || nonfinal.library.LibraryKt.getOrder() != 8451) throw new AssertionError("named order");
        if (ConsumerKt.spread() != 218) throw new AssertionError("spread");
        if (!ConsumerKt.generic().equals("first") || !ConsumerKt.genericEmpty().equals("last")) throw new AssertionError("generic");
        if (ConsumerKt.member() != 6 || ConsumerKt.constructor() != 6) throw new AssertionError("members");
        if (ConsumerKt.extension() != 16) throw new AssertionError("extension");
        if (ConsumerKt.overload() != 3) throw new AssertionError("overload specificity");
        System.out.println("nonfinal-varargs-ok");
    }
}
