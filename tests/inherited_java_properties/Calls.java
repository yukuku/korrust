public final class Calls {
    private static void check(boolean value) { if (!value) throw new AssertionError(); }
    public static void main(String[] args) {
        Child child = new Child();
        check(EffectsKt.read(child).equals("start"));
        EffectsKt.write(child, "next"); check(EffectsKt.read(child).equals("next"));
        check(child.rename("inside").equals("inside"));
        check(child.reads == 3 && child.writes == 2);
        check(child.toggle() && !child.toggle());
        check(EffectsKt.optional(child) == null);
        EffectsKt.writeOptional(child, "optional"); check(EffectsKt.optional(child).equals("optional"));
        EffectsKt.writeOptional(child, null); check(EffectsKt.optional(child) == null);
        check(EffectsKt.url(child).equals("url"));
        check(EffectsKt.overridden(new Overriding(), "name").equals("override:name!"));
        check(EffectsKt.getterOverride(new GetterOverride(), "name").equals("getter:name"));
        KotlinSetter setter = new KotlinSetter(); EffectsKt.kotlinSetter(setter, "recorded");
        check(setter.getRecorded().equals("recorded"));
        check(EffectsKt.generic(new GenericChild(), "generic").equals("generic"));
        System.out.println("OK");
    }
}
