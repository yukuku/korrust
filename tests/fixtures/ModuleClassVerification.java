import java.util.Arrays;

public final class ModuleClassVerification {
    public static void main(String[] names) throws Exception {
        Arrays.sort(names);
        ClassLoader loader = ModuleClassVerification.class.getClassLoader();
        for (String name : names) {
            Class<?> type = Class.forName(name, false, loader);
            type.getDeclaredConstructors();
            type.getDeclaredMethods();
            type.getDeclaredFields();
        }
        System.out.println("Verified " + names.length + " module classes");
    }
}
