import java.lang.reflect.*;
import java.util.*;
import java.util.concurrent.*;
import novjvm.*;
import novjvm.support.CustomFailure;
public final class JavaCaller {
    static void check(boolean ok) { if (!ok) throw new AssertionError(); }
    static void exceptions(Executable method, Class<?>... expected) {
        check(Arrays.equals(method.getExceptionTypes(), expected));
        for (java.lang.annotation.Annotation annotation: method.getDeclaredAnnotations()) {
            String name = annotation.annotationType().getName();
            check(!name.equals("kotlin.jvm.Throws") && !name.equals("kotlin.jvm.Synchronized"));
        }
    }
    static Method method(Class<?> owner, String name) throws Exception { return owner.getDeclaredMethod(name); }
    static void synchronizedFlag(Class<?> owner, String name, boolean expected) throws Exception {
        check(Modifier.isSynchronized(method(owner, name).getModifiers()) == expected);
    }
    static void released(Object target) throws Exception {
        CountDownLatch entered = new CountDownLatch(1);
        Thread thread = new Thread(() -> { synchronized(target) { entered.countDown(); } });
        thread.setDaemon(true);
        thread.start();
        check(entered.await(3, TimeUnit.SECONDS));
        thread.join(3000);
        check(!thread.isAlive());
    }
    public static void main(String[] args) throws Throwable {
        exceptions(method(FunctionsKt.class, "declared"), java.io.IOException.class, CustomFailure.class, Throwable.class);
        exceptions(method(FunctionsKt.class, "named"), java.io.IOException.class, CustomFailure.class);
        exceptions(method(FunctionsKt.class, "spread"), java.io.IOException.class);
        exceptions(method(FunctionsKt.class, "empty"));
        exceptions(method(novjvm.aliases.AliasesKt.class, "qualified"), java.io.IOException.class);
        synchronizedFlag(novjvm.aliases.AliasesKt.class, "qualifiedMonitor", true);
        exceptions(method(FunctionsKt.class, "sourceFailure"), SourceFailure.class);
        exceptions(method(FunctionsKt.class, "defaulted"), java.io.IOException.class);
        exceptions(FunctionsKt.class.getDeclaredMethod("defaulted", int.class), java.io.IOException.class);
        exceptions(FunctionsKt.class.getDeclaredMethod("defaulted$default", int.class, int.class, Object.class), java.io.IOException.class);
        exceptions(method(Defaults.class, "value"), java.io.IOException.class);
        exceptions(Defaults.class.getDeclaredMethod("value", int.class), java.io.IOException.class);
        exceptions(Defaults.class.getDeclaredMethod("value$default", int.class, int.class, Object.class), java.io.IOException.class);
        exceptions(Worker.class.getDeclaredConstructor(), CustomFailure.class);
        exceptions(Constructed.class.getDeclaredConstructor(int.class), java.io.IOException.class);
        exceptions(Constructed.class.getDeclaredConstructor(), java.io.IOException.class);
        exceptions(Constructed.class.getDeclaredConstructor(int.class, int.class, kotlin.jvm.internal.DefaultConstructorMarker.class), java.io.IOException.class);
        exceptions(method(Worker.class, "fail"), java.io.IOException.class);
        exceptions(method(Worker.class, "getHeld"), java.io.IOException.class);
        exceptions(method(Parent.class, "inherited"), java.io.IOException.class);
        exceptions(method(Child.class, "inherited"));
        exceptions(method(Wider.class, "inherited"), Exception.class);
        exceptions(method(Contract.class, "run"), Throwable.class);
        exceptions(method(AbstractTask.class, "work"), java.io.IOException.class);
        exceptions(method(Task.class, "work"), CustomFailure.class);
        check(new Task().work() == 26);
        exceptions(method(Contract.class, "ready"), CustomFailure.class);
        exceptions(Contract.DefaultImpls.class.getDeclaredMethod("ready", Contract.class), CustomFailure.class);
        exceptions(method(Implementation.class, "run"), java.io.IOException.class);
        exceptions(method(Host.ThrowEverythingRunnable.class, "run"), Throwable.class);
        exceptions(method(Host.class, "both"), java.io.IOException.class);
        exceptions(method(Host.Companion.getClass(), "both"), java.io.IOException.class);
        exceptions(method(Singleton.class, "staticMonitor"), CustomFailure.class);
        for (String name: new String[]{"facadeMonitor", "facadeFailure"}) synchronizedFlag(FunctionsKt.class, name, true);
        for (String name: new String[]{"enter", "fail", "getHeld"}) synchronizedFlag(Worker.class, name, true);
        synchronizedFlag(Host.class, "both", true);
        synchronizedFlag(Host.Companion.getClass(), "both", true);
        synchronizedFlag(Singleton.class, "instance", true);
        synchronizedFlag(Singleton.class, "staticMonitor", true);
        synchronizedFlag(Child.class, "inherited", false);
        Worker worker = new Worker();
        check(worker.enter() && worker.getHeld());
        synchronized(worker) { check(worker.enter()); }
        check(FunctionsKt.facadeMonitor());
        check(FunctionsKt.defaulted() == 18 && Defaults.value() == 19);
        check(new Constructed().getValue() == 22);
        check(Singleton.INSTANCE.instance() && Singleton.staticMonitor());
        check(Host.Companion.both() == 1 && Host.both() == 3);
        try { worker.fail(); throw new AssertionError(); } catch (java.io.IOException expected) {}
        check(!Thread.holdsLock(worker));
        released(worker);
        try { FunctionsKt.facadeFailure(); throw new AssertionError(); } catch (java.io.IOException expected) {}
        check(!Thread.holdsLock(FunctionsKt.class));
        released(FunctionsKt.class);
        try { check(FunctionsKt.declared() == 7); } catch (CustomFailure expected) { throw new AssertionError(); }
        check(new Child().inherited() == 12 && new Wider().inherited() == 13);
        Contract contract = new Implementation();
        check(contract.run() == 15 && contract.ready() == 14);
        check(Contract.DefaultImpls.ready(contract) == 14);
        System.out.println("JAVA");
    }
}
