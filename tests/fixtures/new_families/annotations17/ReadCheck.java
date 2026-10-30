package annotation17check;

import annotation17.*;
import annotation17java.JavaRich;
import annotation17reads.ReadsKt;
import kotlin.jvm.internal.Reflection;

public final class ReadCheck {
    private static int count;
    private static void check(boolean value) {
        count++;
        if (!value) throw new AssertionError("read check " + count);
    }
    public static void main(String[] args) {
        Rich defaults = DefaultUse.class.getAnnotation(Rich.class);
        check(ReadsKt.scalar(defaults).equals(Reflection.getOrCreateKotlinClass(Token.class)));
        check(ReadsKt.array(defaults).length == 5);
        check(ReadsKt.array(defaults)[0].equals(Reflection.getOrCreateKotlinClass(int.class)));
        check(ReadsKt.array(defaults)[1].equals(Reflection.getOrCreateKotlinClass(String[].class)));
        check(ReadsKt.array(defaults)[4].equals(Reflection.getOrCreateKotlinClass(Token.class)));
        check(ReadsKt.nested(defaults) == 17L);
        check(ReadsKt.elements(defaults) == 5);
        Rich explicit = ExplicitUse.class.getAnnotation(Rich.class);
        check(ReadsKt.scalar(explicit).equals(Reflection.getOrCreateKotlinClass(int[].class)));
        check(ReadsKt.array(explicit)[2].equals(Reflection.getOrCreateKotlinClass(Token[].class)));
        JavaRich java = annotation17.JavaDefaultUse.class.getAnnotation(JavaRich.class);
        check(ReadsKt.javaScalar(java).equals(Reflection.getOrCreateKotlinClass(int[].class)));
        check(ReadsKt.javaArray(java).length == 2);
        check(ReadsKt.javaArray(java)[0].equals(Reflection.getOrCreateKotlinClass(int.class)));
        check(ReadsKt.javaArray(java)[1].equals(Reflection.getOrCreateKotlinClass(String[].class)));
        try { ReadsKt.scalar(null); throw new AssertionError("missing null check"); }
        catch (NullPointerException expected) { count++; }
        int[] effects = {0};
        check(ReadsKt.supplied(() -> { effects[0]++; return defaults; }).equals(Reflection.getOrCreateKotlinClass(Token.class)));
        check(effects[0] == 1);
        check(ReadsKt.suppliedArray(() -> { effects[0]++; return defaults; }).length == 5);
        check(effects[0] == 2);
        RuntimeException marker = new RuntimeException("supplier");
        try { ReadsKt.supplied(() -> { effects[0]++; throw marker; }); throw new AssertionError("missing exception"); }
        catch (RuntimeException actual) { check(actual == marker); }
        check(effects[0] == 3);
        try { ReadsKt.supplied(() -> { effects[0]++; return null; }); throw new AssertionError("missing receiver null check"); }
        catch (NullPointerException expected) { count++; }
        check(effects[0] == 4);
        kotlin.reflect.KClass<?>[] array = ReadsKt.array(defaults);
        array[0] = null;
        check(ReadsKt.array(defaults)[0] != null);
        System.out.println("OK:" + count);
    }
}
