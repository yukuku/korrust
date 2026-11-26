public class Caller {
    public static void main(String[] args) {
        if (!"value".equals(ConsumerKt.kotlinBox()) || !"word".equals(ConsumerKt.javaRef()) ||
            !"explicit".equals(ConsumerKt.explicit()) || !ConsumerKt.contextual().isEmpty()) throw new AssertionError();
    }
}
