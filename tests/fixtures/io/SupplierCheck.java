import java.lang.reflect.Constructor;
import java.lang.reflect.Array;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.function.Supplier;
import java.util.HashSet;
import java.util.Set;
import java.util.Map;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.Arrays;

public final class SupplierCheck {
    public static void main(String[] names) throws Exception {
        if (names.length > 0 && names[0].equals("--generic")) {
            generic(names);
            return;
        }
        legacy(names);
    }

    private static final class Attempt {
        final Object value;
        final NegativeArraySizeException failure;
        Attempt(Object value, NegativeArraySizeException failure) { this.value = value; this.failure = failure; }
    }

    private static Attempt attempt(Supplier<?> supplier) {
        try { return new Attempt(supplier.get(), null); }
        catch (NegativeArraySizeException failure) { return new Attempt(null, failure); }
    }

    private static String arrayShape(Object value) {
        if (value == null || !value.getClass().isArray()) throw new AssertionError("Supplier did not return an array");
        Class<?> component = value.getClass().getComponentType();
        Object zero = Array.get(Array.newInstance(component, 1), 0);
        for (int index = 0; index < Array.getLength(value); index++) {
            Object element = Array.get(value, index);
            if (zero == null ? element != null : !zero.equals(element)) throw new AssertionError("Array was not zero initialized");
        }
        return value.getClass().getName() + ":" + Array.getLength(value);
    }

    private static void count(Map<String, Integer> inventory, String shape) {
        inventory.put(shape, inventory.getOrDefault(shape, 0) + 1);
    }

    private static String allocation(Supplier<?> supplier) {
        Attempt first = attempt(supplier), second = attempt(supplier);
        if (first.failure != null || second.failure != null) {
            if (first.failure == null || second.failure == null || first.failure == second.failure) throw new AssertionError("Supplier must retry deferred negative allocation");
            return "negative";
        }
        String shape = arrayShape(first.value);
        if (first.value == second.value || !shape.equals(arrayShape(second.value))) throw new AssertionError("Supplier must allocate matching fresh arrays");
        return shape;
    }

    private static String cached(ThreadLocal<?> local, Class<?> expected) {
        local.remove();
        Attempt first = attempt(local::get), second = attempt(local::get);
        if (first.failure != null || second.failure != null) {
            if (first.failure == null || second.failure == null || first.failure == second.failure) throw new AssertionError("ThreadLocal did not retry failed initialization");
            local.remove();
            Attempt third = attempt(local::get);
            if (third.failure == null || third.failure == second.failure) throw new AssertionError("ThreadLocal failure after remove differs");
            return "negative";
        }
        if (first.value == null || first.value.getClass() != expected || first.value != second.value) throw new AssertionError("ThreadLocal cache/type mismatch");
        String shape = arrayShape(first.value);
        local.remove();
        Attempt third = attempt(local::get);
        if (third.failure != null || third.value == first.value || third.value != local.get() || !shape.equals(arrayShape(third.value))) throw new AssertionError("ThreadLocal did not initialize a fresh cache after remove");
        return shape;
    }

    private static void generic(String[] arguments) throws Exception {
        if (arguments.length < 3 || arguments[1].isEmpty()) throw new AssertionError("Expected helper inventory and owner:field=kind specifications");
        Set<String> helperNames = new HashSet<>();
        Map<String, Integer> helperShapes = new HashMap<>(), fieldShapes = new HashMap<>();
        for (String helperName : arguments[1].split(",", -1)) {
            if (helperName.isEmpty() || !helperNames.add(helperName)) throw new AssertionError("Empty or duplicate Supplier helper");
            Class<?> helper = Class.forName(helperName);
            if (!helper.isSynthetic() || !Modifier.isFinal(helper.getModifiers()) || !Supplier.class.isAssignableFrom(helper)) throw new AssertionError("Invalid synthetic Supplier contract: " + helperName);
            if (helper.getDeclaredFields().length != 0) throw new AssertionError("Supplier unexpectedly captures state: " + helperName);
            Constructor<?>[] constructors = helper.getDeclaredConstructors();
            if (constructors.length != 1 || constructors[0].getParameterCount() != 0) throw new AssertionError("Invalid Supplier constructor: " + helperName);
            Method get = helper.getDeclaredMethod("get");
            if (get.getReturnType() != Object.class || get.getParameterCount() != 0 || Modifier.isStatic(get.getModifiers()) || !Modifier.isPublic(get.getModifiers())) throw new AssertionError("Invalid erased Supplier method: " + helperName);
            constructors[0].setAccessible(true);
            count(helperShapes, allocation((Supplier<?>) constructors[0].newInstance()));
        }
        int fields = 0;
        Set<String> ownerNames = new HashSet<>();
        for (String specification : Arrays.copyOfRange(arguments, 2, arguments.length)) {
            String[] parts = specification.split(":", -1);
            if (parts.length != 2 || !ownerNames.add(parts[0])) throw new AssertionError("Invalid or repeated owner specification: " + specification);
            Map<String, Class<?>> expected = new LinkedHashMap<>();
            for (String item : parts[1].split(",", -1)) {
                String[] entry = item.split("=", -1);
                if (entry.length != 2 || entry[0].isEmpty()) throw new AssertionError("Invalid field specification: " + item);
                Class<?> type;
                if (entry[1].equals("byte")) type = byte[].class;
                else if (entry[1].equals("char")) type = char[].class;
                else throw new AssertionError("Unknown expected array element type: " + entry[1]);
                if (expected.put(entry[0], type) != null) throw new AssertionError("Repeated field: " + item);
            }
            Class<?> owner = Class.forName(parts[0]);
            Set<String> actual = new HashSet<>();
            for (Field field : owner.getDeclaredFields()) if (ThreadLocal.class.isAssignableFrom(field.getType())) actual.add(field.getName());
            if (!actual.equals(expected.keySet())) throw new AssertionError("ThreadLocal field inventory differs for " + owner.getName());
            for (Map.Entry<String, Class<?>> entry : expected.entrySet()) {
                Field field = owner.getDeclaredField(entry.getKey());
                int modifiers = field.getModifiers();
                if (field.getType() != ThreadLocal.class || !Modifier.isPrivate(modifiers) || !Modifier.isStatic(modifiers) || !Modifier.isFinal(modifiers)) throw new AssertionError("Invalid ThreadLocal field contract: " + field);
                if (!(field.getGenericType() instanceof ParameterizedType)) throw new AssertionError("Missing ThreadLocal generic signature: " + field);
                ParameterizedType generic = (ParameterizedType) field.getGenericType();
                if (generic.getRawType() != ThreadLocal.class || generic.getActualTypeArguments().length != 1 || generic.getActualTypeArguments()[0] != entry.getValue()) throw new AssertionError("Wrong ThreadLocal array element type: " + field);
                field.setAccessible(true);
                ThreadLocal<?> local = (ThreadLocal<?>) field.get(null);
                if (local == null) throw new AssertionError("ThreadLocal field is null: " + field);
                try {
                    Method bridge = owner.getDeclaredMethod("access$get$" + field.getName() + "$p");
                    if (!bridge.isSynthetic() || !Modifier.isStatic(bridge.getModifiers()) || Modifier.isPublic(bridge.getModifiers()) || Modifier.isProtected(bridge.getModifiers()) || bridge.getReturnType() != ThreadLocal.class || bridge.getParameterCount() != 0) throw new AssertionError("Invalid private field access bridge: " + bridge);
                    bridge.setAccessible(true);
                    if (bridge.invoke(null) != local) throw new AssertionError("Field access bridge changes identity: " + bridge);
                } catch (NoSuchMethodException absent) { }
                count(fieldShapes, cached(local, entry.getValue()));
                fields++;
            }
        }
        if (helperNames.size() != fields || !helperShapes.equals(fieldShapes)) throw new AssertionError("Supplier and ThreadLocal behavior inventories differ: " + helperShapes + " versus " + fieldShapes);
        System.out.println("Generic Supplier lowering verified for " + ownerNames.size() + " objects and " + helperNames.size() + " helpers");
    }

    private static void legacy(String[] names) throws Exception {
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
