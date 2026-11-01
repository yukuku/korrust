package collision;
public class Check {
    public static void main(String[] args) throws Exception {
        Object value = ChoiceKt.f();
        System.out.println(value instanceof String ? value : value.getClass().getName());
        System.out.println(ChoiceKt.class.getMethod("f").getReturnType().getName());
    }
}
