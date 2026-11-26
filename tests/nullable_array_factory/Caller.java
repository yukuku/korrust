import fixtures.*;
public class Caller {
    public static void main(String[] args) {
        Element[] result = ConsumerKt.objects(2);
        if (result.length != 2 || result[1] != null || result.getClass() != Element[].class ||
            ConsumerKt.boxed(1).getClass() != Integer[].class ||
            ConsumerKt.nested().getClass() != String[][].class) throw new AssertionError();
        try { ConsumerKt.objects(-1); throw new AssertionError(); }
        catch (NegativeArraySizeException expected) { }
    }
}
