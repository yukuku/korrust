package annotationboundscheck;
import annotationbounds.*;
import annotationboundsuses.*;
import java.util.Arrays;
public class Check {
    public static void main(String[] args) throws Exception {
        Bounds defaults = Defaults.class.getAnnotation(Bounds.class);
        Bounds explicit = Explicit.class.getAnnotation(Bounds.class);
        if (defaults.error() != IllegalStateException.class || defaults.exact() != Number.class || defaults.lower() != Object.class || defaults.any() != String.class) throw new AssertionError("defaults");
        if (defaults.arrays() != int[].class || defaults.serial() != String[].class || defaults.integer() != Integer.class) throw new AssertionError("class defaults");
        if (explicit.arrays() != int[].class || explicit.serial() != String[].class || explicit.integer() != int.class) throw new AssertionError("class arguments");
        if (kotlin.jvm.JvmClassMappingKt.getJavaClass(AcceptedKt.integerClass(explicit)) != int.class) throw new AssertionError("integer read");
        if (explicit.error() != IllegalArgumentException.class || explicit.exact() != Number.class || explicit.lower() != Object.class || explicit.any() != int.class) throw new AssertionError("explicit");
        if (!Arrays.equals(Numbers.class.getAnnotation(Upper.class).value(), new Class<?>[]{int.class, long.class, Number.class})) throw new AssertionError("array");
        if (DefaultNumbers.class.getAnnotation(Upper.class).value()[0] != Integer.class) throw new AssertionError("array default");
        if (Single.class.getAnnotation(Upper.class).value()[0] != int.class || Spread.class.getAnnotation(Upper.class).value()[0] != long.class) throw new AssertionError("positional");
        if (Empty.class.getAnnotation(Upper.class).value().length != 0) throw new AssertionError("empty");
        if (Recursive.class.getAnnotation(Nested.class).value().value()[0] != double.class) throw new AssertionError("nested");
        if (kotlin.jvm.JvmClassMappingKt.getJavaClass(AcceptedKt.errorClass(explicit)) != IllegalArgumentException.class) throw new AssertionError("read");
        if (kotlin.jvm.JvmClassMappingKt.getJavaClass(AcceptedKt.exactClass(defaults)) != Number.class) throw new AssertionError("exact read");
        if (kotlin.jvm.JvmClassMappingKt.getJavaClass(AcceptedKt.lowerClass(defaults)) != Object.class) throw new AssertionError("lower read");
        if (AcceptedKt.arrayCount(Numbers.class.getAnnotation(Upper.class)) != 3) throw new AssertionError("array read");
        System.out.println("OK:bounds");
    }
}
