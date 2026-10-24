import java.lang.reflect.Constructor;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import kotlin.Metadata;
import kotlinx.serialization.KSerializer;
import kotlinx.serialization.descriptors.SerialDescriptor;
import kotlinx.serialization.descriptors.StructureKind;
import androidx.navigation3.runtime.NavKey;
import com.google.samples.apps.nowinandroid.feature.bookmarks.api.navigation.BookmarksNavKey;
import com.google.samples.apps.nowinandroid.feature.search.api.navigation.SearchNavKey;

public final class SerializationCheck {
    private static int checks;

    private static void require(boolean condition) {
        checks++;
        if (!condition) throw new AssertionError("check " + checks);
    }

    private static void inspect(Class<?> type, Object singleton, KSerializer<?> serializer) throws Exception {
        require(type.getField("INSTANCE").get(null) == singleton);
        require(singleton instanceof NavKey);
        require(Modifier.isFinal(type.getModifiers()));
        Constructor<?>[] constructors = type.getDeclaredConstructors();
        require(constructors.length == 1 && Modifier.isPrivate(constructors[0].getModifiers()));
        Method method = type.getMethod("serializer");
        require(method.getReturnType() == KSerializer.class);
        require(method.getGenericReturnType().getTypeName().equals("kotlinx.serialization.KSerializer<" + type.getName() + ">"));
        require(method.invoke(singleton) == serializer);
        Metadata metadata = type.getAnnotation(Metadata.class);
        require(metadata != null && metadata.k() == 1 && metadata.d1().length > 0 && metadata.d2().length > 0);
        require(type.getAnnotation(kotlinx.serialization.Serializable.class) != null);
        SerialDescriptor descriptor = serializer.getDescriptor();
        require(descriptor.getSerialName().equals(type.getName()));
        require(descriptor.getElementsCount() == 0);
        require(descriptor.getKind() == StructureKind.OBJECT.INSTANCE);
        require(!descriptor.isNullable() && !descriptor.isInline());
        for (int iteration = 0; iteration < 100; iteration++) require(method.invoke(singleton) == serializer);
    }

    private static void concurrentFirstAccess(Object singleton) throws Exception {
        Method method = singleton.getClass().getMethod("serializer");
        ExecutorService pool = Executors.newFixedThreadPool(16);
        CountDownLatch gate = new CountDownLatch(1);
        List<Future<Object>> pending = new ArrayList<>();
        try {
            for (int thread = 0; thread < 16; thread++) {
                pending.add(pool.submit(() -> {
                    gate.await();
                    Object first = method.invoke(singleton);
                    for (int iteration = 0; iteration < 100; iteration++) {
                        if (method.invoke(singleton) != first) throw new AssertionError("racing serializer identity");
                    }
                    return first;
                }));
            }
            gate.countDown();
            Object expected = pending.get(0).get();
            for (Future<Object> result : pending) require(result.get() == expected);
        } finally {
            pool.shutdownNow();
        }
    }

    private static void helper(Class<?> owner) throws Exception {
        Class<?> type = Class.forName(owner.getName() + "$SerializerSupplier");
        require(type.isSynthetic() && Modifier.isFinal(type.getModifiers()));
        require(!Modifier.isPublic(type.getModifiers()) && !Modifier.isProtected(type.getModifiers()));
        require(type.getInterfaces().length == 1 && type.getInterfaces()[0] == kotlin.jvm.functions.Function0.class);
        require(type.getDeclaredFields().length == 1 && Modifier.isStatic(type.getDeclaredField("INSTANCE").getModifiers()));
        require(type.getDeclaredConstructors().length == 1 && Modifier.isPrivate(type.getDeclaredConstructors()[0].getModifiers()));
        java.lang.reflect.Field instance = type.getDeclaredField("INSTANCE");
        require(instance.trySetAccessible());
        kotlin.jvm.functions.Function0<?> supplier = (kotlin.jvm.functions.Function0<?>) instance.get(null);
        Object first = supplier.invoke();
        require(first instanceof KSerializer<?> && first != supplier.invoke());
        require(((KSerializer<?>) first).getDescriptor().getSerialName().equals(owner.getName()));
        require(type.getAnnotation(Metadata.class).k() == 3);
    }

    public static void main(String[] args) throws Exception {
        if (args.length > 0) {
            helper(BookmarksNavKey.class);
            helper(SearchNavKey.class);
            System.out.println("{\"checks\":" + checks + "}");
            return;
        }
        concurrentFirstAccess(BookmarksNavKey.INSTANCE);
        concurrentFirstAccess(SearchNavKey.INSTANCE);
        inspect(BookmarksNavKey.class, BookmarksNavKey.INSTANCE, BookmarksNavKey.INSTANCE.serializer());
        inspect(SearchNavKey.class, SearchNavKey.INSTANCE, SearchNavKey.INSTANCE.serializer());
        require((Object) BookmarksNavKey.INSTANCE != (Object) SearchNavKey.INSTANCE);
        require(BookmarksNavKey.INSTANCE.serializer() != (Object) SearchNavKey.INSTANCE.serializer());
        System.out.println("{\"checks\":" + checks + "}");
    }
}
