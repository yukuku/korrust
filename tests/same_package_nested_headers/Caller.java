import fixture.Container;
import fixture.Consumer;
public class Caller {
    public static void main(String[] args) {
        Consumer c = new Consumer();
        Container.Session s = c;
        Container.Deep.More m = c;
        if (s.id() != 42 || !m.next().equals("ready")) throw new AssertionError();
    }
}
