package nullableannotations;
import java.lang.reflect.Method;
import kotlin.jvm.JvmStatic;
public class JvmStaticReflectionCaller {
    public static void main(String[] args) throws Exception {
        int checked = 0;
        int annotated = 0;
        for (String name : new String[]{"nullableannotations.MethodsKt", "nullableannotations.Member", "nullableannotations.Member$Companion", "nullableannotations.Holder"}) {
            for (Method method : Class.forName(name).getDeclaredMethods()) {
                if (method.isSynthetic()) continue;
                RuntimeMark marker = method.getDeclaredAnnotation(RuntimeMark.class);
                if (marker == null) continue;
                int expected = marker.value().equals("static") || marker.value().equals("companion") ? 1 : 0;
                int actual = method.getDeclaredAnnotationsByType(JvmStatic.class).length;
                if (actual != expected) throw new AssertionError(method + ": JvmStatic count " + actual + " != " + expected);
                checked++;
                annotated += actual;
            }
        }
        if (checked != 21 || annotated != 9) throw new AssertionError(checked + ":" + annotated);
        System.out.println("OK:" + checked + ":" + annotated);
    }
}
