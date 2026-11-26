package mapped;
import java.util.*;
public class Calls {
    public static void main(String[] args) {
        Renamed<String, Integer> map = new Renamed<>();
        map.put("first", 7);
        Set<String> keys = PropertiesKt.keys(map);
        Set<Map.Entry<String, Integer>> entries = PropertiesKt.entries(map);
        if (!keys.equals(Collections.singleton("first"))) throw new AssertionError(keys);
        if (!PropertiesKt.inherited(map).equals(keys)) throw new AssertionError();
        if (PropertiesKt.keyCount(map) != 1) throw new AssertionError();
        if (PropertiesKt.mapSize(map) != 1) throw new AssertionError();
        map.put("second", 9);
        if (keys.size() != 2 || entries.size() != 2) throw new AssertionError("views must remain live");
        if (PropertiesKt.mapSize(map) != 2) throw new AssertionError();
        if (PropertiesKt.collectionSize(keys) != 2) throw new AssertionError();
        if (PropertiesKt.entries(map).iterator().next().getValue() != 7) throw new AssertionError();
        LinkedHashMap<String, Integer> nullable = new LinkedHashMap<>();
        nullable.put("none", null);
        if (PropertiesKt.values(nullable).iterator().next() != null) throw new AssertionError();
        if (!PropertiesKt.values(nullable).contains(null)) throw new AssertionError();
        System.out.println("mapped-properties-ok");
    }
}
