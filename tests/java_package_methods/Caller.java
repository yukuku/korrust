public class Caller {
    public static void main(String[] args) {
        if (app.ConsumerKt.count(new app.Store()) != 13 || new app.Child().own() != 8) throw new AssertionError();
    }
}
