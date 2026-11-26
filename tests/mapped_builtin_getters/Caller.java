public class Caller {
    public static void main(String[] args) {
        Throwable cause = new IllegalStateException();
        Failure failure = new Failure("message", cause);
        if (!ConsumerKt.message(failure).equals("message") || ConsumerKt.cause(failure) != cause
                || ConsumerKt.message(new Failure(null, null)) != null) throw new AssertionError();
    }
}
