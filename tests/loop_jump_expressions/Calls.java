public class Calls {
    public static void main(String[] args) {
        if (JumpsKt.count(4, null) != 46 || JumpsKt.count(4, "z") != 74) {
            throw new AssertionError();
        }
    }
}
