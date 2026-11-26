import store.Box;
import java.util.Map;
public class Caller {
    public static void main(String[] args) {
        Box<Object> box = new Box<>("text");
        if (!"text".equals(ConsumerKt.read(box)) || !"text".equals(ConsumerKt.copy(box).getItem())) throw new AssertionError();
        box.setItem(null);
        if (ConsumerKt.read(box) != null || !ConsumerKt.copyMap(Map.of("k",2)).equals(Map.of("k",2))) throw new AssertionError();
    }
}
