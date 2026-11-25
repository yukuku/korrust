package constantprobe;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
public class Calls {
    static void check(boolean value) { if (!value) throw new AssertionError(); }
    static void constant(Class<?> owner, String name, Object expected) throws Exception {
        Field field = owner.getDeclaredField(name);
        check(Modifier.isStatic(field.getModifiers()));
        check(Modifier.isFinal(field.getModifiers()));
        check(Modifier.isPublic(field.getModifiers()));
        check(field.get(null).equals(expected));
    }
    public static void main(String[] args) throws Exception {
        check(ModelsKt.qualifiedRead() == 4);
        check(ReceiverTrace.INSTANCE.getCount() == 0);
        check(ModelsKt.effectfulRead() == 4);
        check(ReceiverTrace.INSTANCE.getCount() == 11);
        constant(Host.class, "BASE", 25);
        constant(Host.class, "TIMEOUT", 200L);
        constant(Host.class, "LABEL", "hostsettings");
        constant(Host.class, "FLAG", true);
        constant(Host.class, "LETTER", 'x');
        constant(Host.class, "FLOAT", -1.25F);
        constant(Host.class, "DOUBLE", 2.5);
        constant(Host.class, "LARGE", Long.MAX_VALUE);
        constant(Contract.class, "LIMIT", 26);
        constant(Choice.class, "CODE", 27);
        constant(Settings.class, "TOTAL", 30);
        check(new Host().getInitial() == 25);
        check(new Host().getInferred() == 25);
        check(new Host().inferredRead() == 25);
        check(Host.Companion.inferredRead() == 25);
        check(new Host().read() == 23);
        check(new Host().shadow(9) == 10);
        check(new Host().localShadow() == 7);
        check(new Implementation().instanceRead() == 23);
        check(Host.Companion.getRuntime() == 23);
        check(Host.Companion.read() == 21);
        check(Host.Companion.shadow(9) == 11);
        check(Settings.INSTANCE.getRuntime() == 29);
        check(Settings.INSTANCE.read() == 28);
        check(Contract.Companion.read() == 26);
        check(Choice.Companion.read() == 27);
        check(ReadKt.readAll() == 133);
        check(ReadKt.FROM_COMPANION == 25);
        check(ReadKt.FROM_IMPORT == 51);
        for (Class<?> companion : new Class<?>[] {Host.Companion.getClass(), Choice.Companion.getClass()}) {
            for (Field field : companion.getDeclaredFields()) check(!field.getName().equals("BASE") && !field.getName().equals("LIMIT") && !field.getName().equals("CODE"));
            for (java.lang.reflect.Method method : companion.getDeclaredMethods()) check(!method.getName().equals("getBASE") && !method.getName().equals("getLIMIT") && !method.getName().equals("getCODE"));
        }
        System.out.println("OK");
    }
}
