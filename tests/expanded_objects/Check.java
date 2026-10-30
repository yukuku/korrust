import objectcases.*;
import java.lang.reflect.Constructor;
import java.lang.reflect.Modifier;
public final class Check {
 private static int checks;
 private static void check(boolean value) { checks++; if (!value) throw new AssertionError("check " + checks); }
 private static Object duplicate(Class<?> type) throws Exception {
  Constructor<?> constructor = type.getDeclaredConstructor();
  check(Modifier.isPrivate(constructor.getModifiers()));
  constructor.setAccessible(true);
  return constructor.newInstance();
 }
 public static void main(String[] args) throws Exception {
  check(Counter.INSTANCE.getCount() == 0);
  check(Tracked.INSTANCE.getFirst() == 1);
  check(Tracked.INSTANCE.getSecond() == 12);
  check(Tracked.INSTANCE.value() == 13);
  check(Counter.INSTANCE.getCount() == 12);
  check(Tracked.INSTANCE.equals(7));
  check(!Tracked.INSTANCE.equals(8));
  check(ObjectsKt.describe(Start.INSTANCE).equals("start"));
  check(ObjectsKt.describe(Stop.INSTANCE).equals("stop"));
  check(ObjectsKt.describe(Container.Nested.INSTANCE).equals("nested"));
  Object[] values = { Start.INSTANCE, Stop.INSTANCE, Container.Nested.INSTANCE, Empty.INSTANCE, SerializableEntry.INSTANCE };
  String[] names = { "Start", "stop", "Nested", "Empty", "SerializableEntry" };
  for (int i = 0; i < values.length; i++) {
   Object value = values[i];
   Object copy = duplicate(value.getClass());
   check(value != copy);
   check(value.equals(copy));
   check(copy.equals(value));
   check(value.hashCode() == copy.hashCode());
   check(value.hashCode() == value.getClass().getName().replace('$', '.').hashCode());
   check(value.toString().equals(names[i]));
   check(copy.toString().equals(names[i]));
   check(!value.equals(null));
   check(!value.equals(""));
   check(value.getClass().getField("INSTANCE").get(null) == value);
   check(Modifier.isFinal(value.getClass().getModifiers()));
   for (int j = 0; j < values.length; j++) check(value.equals(values[j]) == (i == j));
  }
  check(!Ordinary.INSTANCE.equals(duplicate(Ordinary.class)));
  check(Counter.INSTANCE.getCount() == 12);
  java.io.ByteArrayOutputStream bytes = new java.io.ByteArrayOutputStream();
  try (java.io.ObjectOutputStream output = new java.io.ObjectOutputStream(bytes)) { output.writeObject(SerializableEntry.INSTANCE); }
  Object restored;
  try (java.io.ObjectInputStream input = new java.io.ObjectInputStream(new java.io.ByteArrayInputStream(bytes.toByteArray()))) { restored = input.readObject(); }
  check(restored != SerializableEntry.INSTANCE);
  check(restored.equals(SerializableEntry.INSTANCE));
  check(restored.hashCode() == SerializableEntry.INSTANCE.hashCode());
  System.out.println("OK " + checks);
 }
}
