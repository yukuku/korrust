import java.lang.reflect.Constructor;
import java.lang.reflect.Modifier;
import nextjvm.api.Secret;
import nextjvm.api.ModuleOnly;

public class JavaCaller {
    public static void main(String[] args) throws Exception {
        if (nextjvm.api.StableApi.answer() != 42) throw new AssertionError("facade");
        if (nextjvm.api.StableApi.BASE != 40) throw new AssertionError("constant");
        if (nextjvm.cross.CrossFileKt.crossFile() != 49) throw new AssertionError("cross-file");
        if (Secret.Companion.create(4).again(5).getValue() != 5) throw new AssertionError("factory");
        if (Secret.Companion.create(0).factory().invoke(6).getValue() != 6) throw new AssertionError("lambda");
        if (new Secret.Nested().create(7).getValue() != 7) throw new AssertionError("nested");
        if (new ModuleOnly(8).getValue() != 8) throw new AssertionError("internal JVM constructor");
        if (nextjvm.api.ObjectSecret.Factory.INSTANCE.create() == null) throw new AssertionError("nested object factory");
        Constructor<?> secret = Secret.class.getDeclaredConstructor(int.class);
        if (!Modifier.isPrivate(secret.getModifiers())) throw new AssertionError("private constructor flags");
        Constructor<?> bridge = Secret.class.getDeclaredConstructor(int.class, kotlin.jvm.internal.DefaultConstructorMarker.class);
        if (!Modifier.isPublic(bridge.getModifiers()) || !bridge.isSynthetic()) throw new AssertionError("bridge flags");
        System.out.println("OK");
    }
}
