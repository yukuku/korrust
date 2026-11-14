package nullableannotations;
import java.lang.reflect.Method;
import java.util.Objects;
public class Calls {
    private static int checks;
    private static void expect(Object actual, Object expected) {
        if (!Objects.equals(actual, expected)) throw new AssertionError(actual + " != " + expected);
        checks++;
    }
    private static void call(Class<?> owner, Object receiver, String name, String marker) throws Exception {
        Method full = owner.getDeclaredMethod(name, String.class, boolean.class);
        Method one = owner.getDeclaredMethod(name, String.class);
        Method zero = owner.getDeclaredMethod(name);
        expect(full.invoke(receiver, null, false), null);
        expect(full.invoke(receiver, "value", false), "value");
        expect(full.invoke(receiver, null, true), "forced");
        expect(one.invoke(receiver, "one"), "one");
        expect(zero.invoke(receiver), null);
        for (Method method : new Method[]{full, one, zero}) {
            expect(method.getDeclaredAnnotation(RuntimeMark.class).value(), marker);
            expect(method.getDeclaredAnnotation(BinaryMark.class), null);
        }
    }
    public static void main(String[] args) throws Exception {
        call(MethodsKt.class, null, "top", "top");
        call(Member.class, new Member(), "member", "member");
        call(Holder.class, null, "static", "static");
        call(Holder.class, Holder.INSTANCE, "instance", "instance");
        call(Member.class, null, "companion", "companion");
        call(Member.Companion.class, Member.Companion, "companion", "companion");
        call(Member.Companion.class, Member.Companion, "plain", "plain");
        System.out.println("OK:" + checks);
    }
}
