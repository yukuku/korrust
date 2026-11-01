import java.lang.reflect.*;
import java.util.*;

public final class AbiDump {
    static String type(Class<?> type) { return type.descriptorString(); }
    static String parameters(Class<?>[] values) { StringBuilder out = new StringBuilder("("); for (Class<?> value : values) out.append(type(value)); return out.append(")").toString(); }
    static boolean visibleSynthetic(Method method) { return method.getName().contains("-impl") || method.getName().contains("-impl0") || method.getName().contains("-$default") || method.getName().endsWith("$default") && method.getName().contains("-"); }
    public static void main(String[] args) throws Exception {
        List<String> result = new ArrayList<>();
        for (String owner : args) {
            Class<?> cls = Class.forName(owner, false, AbiDump.class.getClassLoader());
            result.add(owner + " CLASS " + cls.getModifiers());
            for (Class<?> iface : cls.getInterfaces()) result.add(owner + " INTERFACE " + iface.getName());
            for (Field field : cls.getDeclaredFields()) if (!field.isSynthetic()) result.add(owner + " FIELD " + field.getName() + " " + type(field.getType()) + " " + field.getModifiers() + " " + field.getGenericType().getTypeName());
            for (Constructor<?> constructor : cls.getDeclaredConstructors()) {
                Class<?>[] params = constructor.getParameterTypes();
                boolean marker = params.length > 0 && params[params.length - 1].getName().equals("kotlin.jvm.internal.DefaultConstructorMarker");
                if (!constructor.isSynthetic() || marker) result.add(owner + " CTOR " + parameters(params) + "V " + constructor.getModifiers() + " " + Arrays.toString(constructor.getGenericParameterTypes()));
            }
            for (Method method : cls.getDeclaredMethods()) if ((!method.isSynthetic() && (Modifier.isPublic(method.getModifiers()) || Modifier.isProtected(method.getModifiers()))) || visibleSynthetic(method)) result.add(owner + " METHOD " + method.getName() + parameters(method.getParameterTypes()) + type(method.getReturnType()) + " " + method.getModifiers() + " " + method.getGenericReturnType().getTypeName() + " " + Arrays.toString(method.getGenericParameterTypes()));
        }
        Collections.sort(result);
        for (String line : result) System.out.println(line);
    }
}
