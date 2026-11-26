public class Caller {
    public static void main(String[] args) {
        if (new Host().compute() != 7) throw new AssertionError();
    }
}
