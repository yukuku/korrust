import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.function.Supplier;
import java.util.HashSet;
import java.util.Set;

public final class SupplierCheck {
    public static void main(String[] names) throws Exception {
        for (String name : names) {
            Class<?> owner = Class.forName(name);
            Set<String> bridges = new HashSet<>();
            for (Field field : owner.getDeclaredFields()) {
                if (field.getType() != ThreadLocal.class) continue;
                String helperName = name + "$" + field.getName() + "$Supplier";
                Class<?> helper = Class.forName(helperName);
                if (!helper.isSynthetic() || !Modifier.isFinal(helper.getModifiers()) || !Supplier.class.isAssignableFrom(helper)) throw new AssertionError("Invalid synthetic Supplier contract: " + helperName);
                if (helper.getDeclaredFields().length != 0) throw new AssertionError("Supplier unexpectedly captures state: " + helperName);
                Constructor<?>[] constructors = helper.getDeclaredConstructors();
                if (constructors.length != 1 || constructors[0].getParameterCount() != 0) throw new AssertionError("Invalid Supplier constructor: " + helperName);
                bridges.add("access$" + field.getName() + "$lambda$0");
                Method bridge = owner.getDeclaredMethod("access$" + field.getName() + "$lambda$0");
                if (!bridge.isSynthetic() || !Modifier.isStatic(bridge.getModifiers()) || bridge.getParameterCount() != 0) throw new AssertionError("Invalid Supplier access bridge: " + helperName);
                Method method = helper.getDeclaredMethod("get");
                if (method.getReturnType() != Object.class || method.getParameterCount() != 0) throw new AssertionError("Invalid erased Supplier method: " + helperName);
                constructors[0].setAccessible(true);
                Supplier<?> supplier = (Supplier<?>)constructors[0].newInstance();
                Object first = null, second = null;
                String firstFailure = "success", secondFailure = "success";
                try { first = supplier.get(); } catch (NegativeArraySizeException error) { firstFailure = error.getClass().getName(); }
                try { second = supplier.get(); } catch (NegativeArraySizeException error) { secondFailure = error.getClass().getName(); }
                if (!firstFailure.equals(secondFailure)) throw new AssertionError("Supplier failure changed");
                if (firstFailure.equals("success") && (first == second || first == null || !first.getClass().isArray() || first.getClass() != bridge.getReturnType())) throw new AssertionError("Supplier must allocate a fresh array per call");
                field.setAccessible(true);
                ThreadLocal<?> local = (ThreadLocal<?>)field.get(null);
                if (firstFailure.equals("success") && local.get() != local.get()) throw new AssertionError("ThreadLocal cache identity mismatch");
            }
            for (Method method : owner.getDeclaredMethods()) {
                if (method.getName().startsWith("access$") && method.getName().endsWith("$lambda$0") && !bridges.contains(method.getName())) throw new AssertionError("Unexpected Supplier access bridge");
            }
        }
        System.out.println("Synthetic Supplier lowering verified for " + names.length + " objects");
    }
}
