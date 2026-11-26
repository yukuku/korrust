import fixture.*;
public class Caller {
    public static void main(String[] args) {
        Dimensions value = new Dimensions();
        if (ConsumerKt.increment(value) != 20 || value.width != 10 || Dimensions.count != 20) throw new AssertionError();
    }
}
