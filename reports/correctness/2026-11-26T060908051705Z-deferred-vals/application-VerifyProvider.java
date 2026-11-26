public class VerifyProvider {
    public static void main(String[] args) throws Exception {
        for (String name : args) {
            Class<?> type = Class.forName(name, false, VerifyProvider.class.getClassLoader());
            type.getDeclaredConstructors();
            type.getDeclaredMethods();
            type.getDeclaredFields();
        }
        System.out.println("provider-classes-verified");
    }
}
