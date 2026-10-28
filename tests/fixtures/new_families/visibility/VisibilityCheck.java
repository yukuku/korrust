package visibilityprobe;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
public final class VisibilityCheck {
 private static int checks;
 private static void require(boolean value) { checks++; if (!value) throw new AssertionError("visibility check=" + checks); }
 private static void absentSetter(Class<?> type) throws Exception {
  try { type.getDeclaredMethod("setValue", int.class); throw new AssertionError("unexpected setter: " + type); }
  catch (NoSuchMethodException expected) { require(true); }
 }
 public static final class Inherited extends ProtectedDefault {
  public void update(int value) { setValue(value); }
 }
 public static void main(String[] args) throws Exception {
  PrivateDefault value = new PrivateDefault();
  require(value.getValue() == 1);
  value.assign(3);
  require(value.getValue() == 3);
  value.deferred(7).invoke();
  require(value.getValue() == 7);
  absentSetter(PrivateDefault.class);
  absentSetter(ExplicitGetter.class);
  absentSetter(State.class);
  absentSetter(Host.Companion.getClass());
  CustomPrivate custom = new CustomPrivate();
  custom.assign(9);
  require(custom.getValue() == 10);
  Method customSetter = CustomPrivate.class.getDeclaredMethod("setValue", int.class);
  require(Modifier.isPrivate(customSetter.getModifiers()));
  customSetter.setAccessible(true);
  customSetter.invoke(custom, 13);
  require(custom.getValue() == 14);
  Inherited inherited = new Inherited();
  inherited.update(17);
  require(inherited.getValue() == 17);
  require(Modifier.isProtected(ProtectedDefault.class.getDeclaredMethod("setValue", int.class).getModifiers()));
  InternalDefault internal = new InternalDefault();
  internal.setValue$visibility_feature(23);
  require(internal.getValue$visibility_feature() == 23);
  PublicInternalSetter mixed = new PublicInternalSetter();
  mixed.setValue$visibility_feature(29);
  require(mixed.getValue() == 29);
  State.INSTANCE.assign(31);
  require(State.INSTANCE.getValue() == 31);
  State.INSTANCE.setHidden$visibility_feature(37);
  require(State.INSTANCE.getHidden$visibility_feature() == 37);
  Host.Companion.assign(41);
  require(Host.Companion.getValue() == 41);
  Host.Companion.setHidden$visibility_feature(43);
  require(Host.Companion.getHidden$visibility_feature() == 43);
  require(VisibilityKt.getTop() == 1);
  OtherTarget first = new OtherTarget();
  OtherTarget second = new OtherTarget();
  first.assignOther(second, 47);
  require(first.getValue() == 0 && second.getValue() == 47);
  first.deferredOther(second, 53).invoke();
  require(first.getValue() == 0 && second.getValue() == 53);
  absentSetter(OtherTarget.class);
  CustomOther customFirst = new CustomOther();
  CustomOther customSecond = new CustomOther();
  customFirst.assignOther(customSecond, 59);
  require(customFirst.getValue() == 0 && customSecond.getValue() == 60);
  customFirst.deferredOther(customSecond, 61).invoke();
  require(customFirst.getValue() == 0 && customSecond.getValue() == 62);
  SetterName named = new SetterName();
  named.setValue(67);
  require(named.getValue() == 68);
  Effects.trace = "";
  Effects.failValue = true;
  VisibilityKt.safeAssign(null, 71);
  require(Effects.trace.equals("R"));
  SafeTarget safe = new SafeTarget();
  Effects.trace = "";
  Effects.failValue = false;
  VisibilityKt.safeAssign(safe, 73);
  require(Effects.trace.equals("RVS") && safe.getValue() == 73);
  Effects.trace = "";
  Effects.failValue = true;
  try { VisibilityKt.safeAssign(safe, 79); throw new AssertionError("missing RHS failure"); }
  catch (RuntimeException failure) { require(failure == Effects.FAILURE && Effects.trace.equals("RV") && safe.getValue() == 73); }
  Effects.failValue = false;
  visibilityqualified.Outer.Entry entry = new visibilityqualified.Outer.Entry(83);
  require(new visibilityqualified.Outer().keep(entry) == entry && entry.getValue() == 83);
  StringBuilder builder = new StringBuilder("qualified");
  require(visibilityqualified.QualifiedKt.keep(builder) == builder);
  Contract.Companion.assign(89);
  require(Contract.Companion.getValue() == 89);
  Contract.Companion.setHidden$visibility_feature(97);
  require(Contract.Companion.getHidden$visibility_feature() == 97);
  absentSetter(Contract.Companion.getClass());
  WideTarget wideFirst = new WideTarget();
  WideTarget wideSecond = new WideTarget();
  wideFirst.assignOther(wideSecond, Long.MIN_VALUE);
  require(wideFirst.getValue() == 1L && wideSecond.getValue() == Long.MIN_VALUE);
  wideFirst.deferredOther(wideSecond, Long.MAX_VALUE).invoke();
  require(wideSecond.getValue() == Long.MAX_VALUE);
  require(java.util.Arrays.stream(WideTarget.class.getDeclaredMethods()).noneMatch(method -> method.getName().equals("setValue")));
  NullableTarget nullableFirst = new NullableTarget();
  NullableTarget nullableSecond = new NullableTarget();
  nullableFirst.assignOther(nullableSecond, "nullable");
  require(nullableFirst.getValue() == null && nullableSecond.getValue().equals("nullable"));
  nullableFirst.assignOther(nullableSecond, null);
  require(nullableSecond.getValue() == null);
  require(VisibilityKt.at(java.util.Arrays.asList("a", "b"), 1).equals("b"));
  require(VisibilityKt.at(java.util.Arrays.asList("a", "b"), -1) == null);
  require(VisibilityKt.mutableAt(new java.util.ArrayList<String>(), 0) == null);
  require(AliasedExtensionKt.aliasAt(java.util.Arrays.asList("alias"), 0).equals("alias"));
  java.util.UUID token = new java.util.UUID(1L, 2L);
  require(visibilityqualified.QualifiedKt.keepToken(token) == token);
  visibilityshadow.Direct direct = new visibilityshadow.Direct();
  require(direct.getValue() == 0);
  require(Modifier.isPrivate(visibilityshadow.Direct.class.getDeclaredField("value").getModifiers()));
  require(visibilityshadow.Direct.class.getDeclaredField("value").getAnnotation(visibilityshadow.JvmField.class) != null);
  absentSetter(visibilityshadow.Direct.class);
  visibilityshadowuse.Aliased aliased = new visibilityshadowuse.Aliased();
  require(aliased.getValue() == 0);
  require(Modifier.isPrivate(visibilityshadowuse.Aliased.class.getDeclaredField("value").getModifiers()));
  require(visibilityshadowuse.Aliased.class.getDeclaredField("value").getAnnotation(visibilityshadow.JvmField.class) != null);
  absentSetter(visibilityshadowuse.Aliased.class);
  require(visibilityexplicit.ExplicitFieldKt.write(137) == 137);
  require(FakeCompanion.Companion.getValue() == 137);
  System.out.println("checks=" + checks);
 }
}
