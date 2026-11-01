package enums.initialization;

public final class Calls {
    static void expect(Object wanted, Object actual) {
        if (!java.util.Objects.equals(wanted, actual)) throw new AssertionError(wanted + " != " + actual);
    }
    static void initialization(String name, String message) throws Exception {
        try {
            Class.forName("enums.initialization." + name);
            throw new AssertionError("initialization succeeded");
        } catch (ExceptionInInitializerError error) {
            expect(IllegalArgumentException.class, error.getCause().getClass());
            expect(message, error.getCause().getMessage());
        }
        try {
            Class.forName("enums.initialization." + name);
            throw new AssertionError("failed initialization repeated");
        } catch (NoClassDefFoundError expected) {
            expect(ExceptionInInitializerError.class, expected.getCause().getClass());
        }
    }
    public static void main(String[] args) throws Exception {
        initialization("BadDefault", "default");
        expect("default", Events.INSTANCE.getText());
        Events.INSTANCE.setText("");
        initialization("BadBody", "body");
        expect("argumentbody", Events.INSTANCE.getText());
        NullableAction action = Action.A;
        expect(null, action.apply(null));
        expect("value!", action.apply("value"));
        try {
            action.apply("fail");
            throw new AssertionError("override did not throw");
        } catch (IllegalStateException expected) {
            expect("fail", expected.getMessage());
        }
        System.out.println("OK");
    }
}
