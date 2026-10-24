import java.lang.reflect.*;
import java.util.*;
import twenty.*;
import twenty.dep.*;
import yuku.alkitab.base.audio.*;
import yuku.alkitab.songs.SongPlaybackState;

public final class TwentyCheck {
    interface Action { void run() throws Exception; }
    static int checks;
    static void eq(Object actual, Object expected) { checks++; if (!Objects.equals(actual, expected)) throw new AssertionError("expected " + expected + ", got " + actual); }
    static void yes(boolean value) { eq(value, true); }
    static void fails(Class<? extends Throwable> type, Action action) throws Exception {
        checks++;
        try { action.run(); } catch (Throwable error) {
            if (error instanceof InvocationTargetException) error = error.getCause();
            if (type.isInstance(error)) return;
            throw new AssertionError(type.getName(), error);
        }
        throw new AssertionError("missing " + type.getName());
    }
    static void observe(String id, Action action) throws Exception {
        int start = checks; action.run();
        System.out.println("{\"id\":\"" + id + "\",\"outcome\":\"pass\",\"checks\":" + (checks - start) + "}");
    }
    static Object maskDefault() throws Exception {
        Class<?>[] types = new Class<?>[34]; Object[] values = new Object[34];
        for (int i = 0; i < 32; i++) { types[i] = i == 0 || i == 15 ? long.class : int.class; values[i] = types[i] == long.class ? (Object)Long.valueOf(0) : Integer.valueOf(0); }
        types[32] = int.class; values[32] = -1; types[33] = Object.class;
        Method method = Mask32Kt.class.getDeclaredMethod("mask32$default", types);
        yes(method.isSynthetic());
        return method.invoke(null, values);
    }
    static Object maskConstructor() throws Exception {
        Class<?>[] types = new Class<?>[34]; Object[] values = new Object[34];
        for (int i = 0; i < 32; i++) { types[i] = i == 0 || i == 15 ? long.class : int.class; values[i] = types[i] == long.class ? (Object)Long.valueOf(0) : Integer.valueOf(0); }
        types[32] = int.class; values[32] = -1; types[33] = kotlin.jvm.internal.DefaultConstructorMarker.class;
        Constructor<?> constructor = MaskCtor.class.getDeclaredConstructor(types);
        yes(constructor.isSynthetic());
        return constructor.newInstance(values);
    }
    public static void main(String[] args) throws Exception {
        observe("top_level_constants", () -> {
            eq(ConstsKt.class.getField("ANSWER").get(null), 42); eq(ConstsKt.class.getField("BIG").get(null), Long.MAX_VALUE);
            eq(ConstsKt.class.getField("FRACTION").get(null), 1.25f); eq(Float.floatToRawIntBits(ConstsKt.class.getField("NEGATIVE_ZERO").getFloat(null)), Integer.MIN_VALUE);
            eq(ConstsKt.class.getField("LABEL").get(null), "\u0000\ud800😀"); eq(ConstsKt.hiddenConstant(), -3);
            yes(Modifier.isFinal(ConstsKt.class.getField("ANSWER").getModifiers()));
        });
        observe("java_static_calls", () -> {
            Trace.initializations = 0; eq(OpsKt.constantRead(), 17); eq(Trace.initializations, 0); eq(OpsKt.staticCall(4, 8), 12); eq(Trace.initializations, 1);
            eq(OpsKt.staticCall(Integer.MAX_VALUE, 1), Integer.MIN_VALUE); eq(Trace.initializations, 1);
            Trace.events = ""; fails(IllegalArgumentException.class, () -> OpsKt.staticThrow()); eq(Trace.events, "T");
        });
        observe("java_fields", () -> {
            JavaApi api = new JavaApi(); eq(OpsKt.staticRead(), 5); eq(OpsKt.staticWrite(19), 19); eq(JavaApi.changing, 19);
            eq(OpsKt.fieldRead(api), 30); eq(OpsKt.fieldWrite(api, -7), -7); eq(api.mutable, -7);
            yes(Modifier.isVolatile(JavaApi.class.getField("changing").getModifiers()));
            yes(Modifier.isVolatile(JavaApi.class.getField("mutable").getModifiers())); yes(Modifier.isFinal(JavaApi.class.getField("immutable").getModifiers()));
            fails(NullPointerException.class, () -> OpsKt.fieldRead(null));
            Trace.events = ""; eq(OpsKt.fieldReceiver(), 7); eq(Trace.events, "Q");
            Trace.events = ""; eq(OpsKt.fieldReceiverWrite(12), 12); eq(Trace.events, "Q");
            Trace.events = ""; fails(NullPointerException.class, () -> OpsKt.instanceConstantNull()); eq(Trace.events, "R");
        });
        observe("overload_resolution", () -> {
            Trace.events = ""; eq(OpsKt.orderedCall(), 5); eq(Trace.events, "RABM");
            Trace.events = ""; fails(NullPointerException.class, () -> OpsKt.orderedNullCall()); eq(Trace.events, "RAB");
            eq(OpsKt.javaLiteral(), "int"); eq(OpsKt.javaLong(), "long"); eq(OpsKt.javaNull(), "string");
            eq(OpsKt.kotlinLiteral(), "int"); eq(OpsKt.kotlinLong(), "long"); eq(OpsKt.kotlinNull(), "string");
        });
        observe("implicit_constructors", () -> { eq(new Implicit().value(), 11); eq(ConstructionKt.implicitConstruct(), 11); });
        observe("nonproperty_constructor_parameters", () -> {
            Trace.events = ""; Captured item = new Captured(4, 20L); eq(item.total(), "20:5"); eq(Trace.events, "ABC");
            Set<String> names = new HashSet<>(); for (Field field : Captured.class.getDeclaredFields()) names.add(field.getName());
            eq(names, Set.of("first", "second", "third"));
        });
        observe("mutable_constructor_properties", () -> {
            MutableCtor item = new MutableCtor(2, null, 5L); eq(item.getText(), null); item.setText("new"); eq(item.getText(), "new");
            item.setCount(8); eq(item.getCount(), 8); eq(item.update(9), 9); eq(item.updateHidden(Long.MIN_VALUE), Long.MIN_VALUE);
            yes(Modifier.isPrivate(MutableCtor.class.getDeclaredField("hidden").getModifiers()));
        });
        observe("initialized_instance_properties", () -> {
            Trace.events = ""; Initialized item = new Initialized(6); eq(Trace.events, "123"); eq(item.getCount(), 6); eq(item.getNext(), 7); eq(item.getLabel(), "ready");
            item.setCount(20); eq(item.change(30), 7); eq(item.getCount(), 30); eq(Trace.events, "123");
        });
        observe("source_construction_named_arguments", () -> {
            Trace.events = ""; Captured item = ConstructionKt.explicitConstruct(); eq(Trace.events, "WSABC"); eq(item.total(), "20:3");
            Trace.events = ""; DefaultCtor defaults = ConstructionKt.namedConstruct(); eq(Trace.events, "CAbi"); eq(defaults.getA(), 4); eq(defaults.getB(), 3L); eq(defaults.getC(), 9);
        });
        observe("function_constructor_defaults", () -> {
            Trace.events = ""; eq(DefaultsKt.namedDefaults(), "4:3:9"); eq(Trace.events, "CAb");
            Trace.events = ""; DefaultCtor item = new DefaultCtor(); eq(Trace.events, "abci"); eq(item.getA(), 2); eq(item.getB(), 3L); eq(item.getC(), 3);
            Trace.events = ""; eq(ConstructionKt.importedMemberDefault(), "3:8"); eq(Trace.events, "Ba");
            Method memberBridge = DefaultMethods.class.getDeclaredMethod("compute$default", DefaultMethods.class, long.class, int.class, int.class, Object.class);
            yes(memberBridge.isSynthetic()); Trace.events = ""; eq(memberBridge.invoke(null, new DefaultMethods(), 0L, 0, 3, null), "3:4"); eq(Trace.events, "ab");
            eq(DefaultsKt.invokePrivateDefault(), 8); eq(Mask32Kt.invokeMask32(), "1:16:90"); eq(maskDefault(), "1:16:32"); eq(((MaskCtor)maskConstructor()).total(), "1:16:32");
            Method bridge = DefaultsKt.class.getDeclaredMethod("plainDefault$default", int.class, int.class, Object.class);
            yes(bridge.isSynthetic()); eq(bridge.invoke(null, 99, 1, null), 7); eq(bridge.invoke(null, 99, 0, null), 99);
        });
        observe("companion_initialization", () -> {
            Trace.events = ""; eq(CompanionInit.Companion.getFIRST(), 4); eq(CompanionInit.Companion.getSECOND(), 6);
            yes(CompanionInit.Companion.getINSTANCE() == CompanionInit.Companion.getINSTANCE()); eq(CompanionInit.Companion.getINSTANCE().getValue(), 6); eq(Trace.events, "XY");
            Field field = CompanionInit.class.getDeclaredField("INSTANCE"); yes(Modifier.isPrivate(field.getModifiers())); yes(Modifier.isStatic(field.getModifiers()));
        });
        observe("nullable_list_member_signatures", () -> {
            List<AudioLogEntry> logs = new ArrayList<>(); Signatures value = new Signatures(null, logs);
            MemberSurface surface = new MemberImplementation(logs); eq(surface.nullable(null), null); eq(surface.nullable(10000), 10000); yes(surface.logs() == logs);
            yes(value.entries(logs) == logs); eq(value.nullable(null), null); eq(value.count(500), 500); eq(value.getNullableList(), Arrays.asList((Integer)null));
            eq(Signatures.class.getDeclaredMethod("entries", List.class).getGenericReturnType().getTypeName(), "java.util.List<yuku.alkitab.base.audio.AudioLogEntry>");
        });
        observe("nullable_primitive_boxing", () -> {
            for (int value : new int[] {-129, -128, 0, 127, 128, 10000, Integer.MIN_VALUE, Integer.MAX_VALUE}) { eq(OpsKt.boxInt(value), value); eq(OpsKt.boxedInt(Integer.valueOf(value)), value); }
            eq(OpsKt.unboxInt(10000), 10000); eq(OpsKt.unboxLong(Long.MIN_VALUE), Long.MIN_VALUE);
            fails(NullPointerException.class, () -> OpsKt.unboxInt(null)); fails(NullPointerException.class, () -> OpsKt.unboxLong(null));
            eq(OpsKt.boxedInt(null), null); eq(OpsKt.boxedLong(null), null); eq(OpsKt.boxedBoolean(null), null); eq(OpsKt.boxedChar(null), null); eq(OpsKt.boxedByte(null), null); eq(OpsKt.boxedFloat(null), null);
            eq(OpsKt.boxLong(Long.MIN_VALUE), Long.MIN_VALUE); eq(OpsKt.boxedLong(Long.MAX_VALUE), Long.MAX_VALUE);
            eq(OpsKt.boxedBoolean(true), true); eq(OpsKt.boxedChar('\ud800'), '\ud800'); eq(OpsKt.boxedByte((byte)-128), (byte)-128);
            eq(OpsKt.boxFloat(-0.0f), -0.0f); yes(Float.isNaN(OpsKt.boxFloat(Float.NaN)));
        });
        observe("primitive_safe_calls", () -> {
            eq(OpsKt.nullableText(null), null); eq(OpsKt.nullableText(10000), "10000");
            eq(OpsKt.nullableLongText(null), null); eq(OpsKt.nullableLongText(Long.MIN_VALUE), "-9223372036854775808");
            eq(OpsKt.chainedPrimitive(null), null); eq(OpsKt.chainedPrimitive("😀"), "2");
            eq(OpsKt.safeLength(null), null); eq(OpsKt.safeLength("😀"), 2); eq(OpsKt.safeEmpty(null), null); eq(OpsKt.safeEmpty(""), true);
            Trace.events = ""; eq(OpsKt.safeArgument(null), null); eq(Trace.events, "");
            eq(OpsKt.safeArgument(new JavaApi()), 9); eq(Trace.events, "AM");
            eq(OpsKt.primitiveElvis(null), -1); eq(OpsKt.primitiveElvis("abc"), 3);
        });
        observe("stable_local_smartcasts", () -> {
            for (String value : new String[] {null, "", "😀"}) { int expected = value == null ? -1 : value.length(); eq(OpsKt.smart(value), expected); eq(OpsKt.smartEarly(value), expected); eq(OpsKt.smartLocal(value), expected); }
        });
        observe("abrupt_elvis", () -> {
            Trace.events = ""; eq(OpsKt.elvisReturn(null), -1); eq(Trace.events, "R"); eq(OpsKt.elvisReturn("abcd"), 4); eq(Trace.events, "R");
            Trace.events = ""; fails(IllegalArgumentException.class, () -> OpsKt.elvisThrow(null)); eq(Trace.events, "E"); eq(OpsKt.elvisThrow("x"), 1); eq(Trace.events, "E");
        });
        observe("typed_lists", () -> {
            AudioLogEntry entry = new AudioLogEntry(1L, "a"); yes(OpsKt.appList(Arrays.asList(entry)) == entry); eq(OpsKt.intList(Arrays.asList(10000)), 10000);
            eq(OpsKt.longList(Arrays.asList(Long.MIN_VALUE)), Long.MIN_VALUE); eq(OpsKt.floatList(Arrays.asList(-0.0f)), -0.0f); eq(OpsKt.nullableList(Arrays.asList((Integer)null)), null);
            eq(OpsKt.booleanList(Arrays.asList(true)), true); eq(OpsKt.byteList(Arrays.asList((byte)-128)), (byte)-128); eq(OpsKt.charList(Arrays.asList('\ud800')), '\ud800');
            eq(OpsKt.intList((List)Arrays.asList(4294967297L)), 1); eq(OpsKt.intList((List)Arrays.asList(-4.5)), -4);
            eq(OpsKt.longList((List)Arrays.asList(1.5f)), 1L); eq(OpsKt.floatList((List)Arrays.asList(-0.0)), -0.0f);
            eq(OpsKt.byteList((List)Arrays.asList(130)), (byte)-126);
            eq(OpsKt.nullableListAssert((List)Arrays.asList(4294967297L)), 1);
            fails(ClassCastException.class, () -> OpsKt.nullableList((List)Arrays.asList(4294967297L)));
            fails(ClassCastException.class, () -> OpsKt.nullableListLocal((List)Arrays.asList(4294967297L)));
            fails(ClassCastException.class, () -> OpsKt.nullableListElvis((List)Arrays.asList(4294967297L)));
            fails(NullPointerException.class, () -> OpsKt.nullableListAssert(Arrays.asList((Integer)null)));
            fails(NullPointerException.class, () -> OpsKt.nullableListLocal(Arrays.asList((Integer)null)));
            eq(OpsKt.nullableListElvis(Arrays.asList((Integer)null)), 0);
            fails(ClassCastException.class, () -> OpsKt.charList((List)Arrays.asList(65))); fails(ClassCastException.class, () -> OpsKt.booleanList((List)Arrays.asList(1)));
            fails(IndexOutOfBoundsException.class, () -> OpsKt.intList(Collections.emptyList()));
            fails(ClassCastException.class, () -> OpsKt.intList((List)Arrays.asList("wrong")));
            fails(NullPointerException.class, () -> OpsKt.intList(Arrays.asList((Integer)null)));
        });
        observe("list_factories_mutation", () -> {
            eq(OpsKt.empty(), Collections.emptyList()); eq(OpsKt.scalars(), Arrays.asList(1, 2, 3)); eq(OpsKt.nullableScalars(), Arrays.asList(null, 200));
            fails(UnsupportedOperationException.class, () -> OpsKt.empty().add(new AudioLogEntry(1L, "x")));
            List<Integer> scalars = OpsKt.scalars(); eq(scalars.set(0, 9), 1); eq(scalars, Arrays.asList(9, 2, 3));
            fails(ArrayStoreException.class, () -> ((List)scalars).set(0, "wrong"));
            List<AudioLogEntry> apps = OpsKt.appFactory(new AudioLogEntry(1L, "x")); fails(ArrayStoreException.class, () -> ((List)apps).set(0, "wrong"));
            eq(OpsKt.booleanFactory(), Arrays.asList(true, false)); eq(OpsKt.byteFactory(), Arrays.asList((byte)-128, (byte)127)); eq(OpsKt.charFactory(), Arrays.asList('\ud800', '\udfff'));
            List<Integer> singleton = OpsKt.singleton(); eq(singleton, Arrays.asList(200));
            fails(UnsupportedOperationException.class, () -> singleton.set(0, 201)); fails(UnsupportedOperationException.class, () -> singleton.add(201));
            List<Integer> zero = OpsKt.mutableEmpty(); zero.add(300); eq(zero, Arrays.asList(300));
            List<Integer> one = OpsKt.mutableSingle(); one.set(0, 300); one.add(400); eq(one, Arrays.asList(300, 400));
            List<Integer> value = OpsKt.mutable(); eq(OpsKt.mutate(value), 17); eq(value, Arrays.asList(8, 4, 9)); value.add(10); eq(value.size(), 4);
            List raw = new ArrayList(Arrays.asList(4294967297L)); eq(OpsKt.replace(raw, 8), 1); eq(raw.get(0), 8);
        });
        observe("rich_data_properties", () -> {
            List<AudioLogEntry> logs = Arrays.asList(new AudioLogEntry(2L, "event"));
            for (float score : new float[] {Float.NaN, Float.intBitsToFloat(0x7f800001), -0.0f, 0.0f, 1.5f}) {
                RichRecord a = new RichRecord(1000, null, score, logs), b = new RichRecord(1000, null, score, new ArrayList<>(logs));
                yes(a.equals(b)); eq(a.hashCode(), b.hashCode()); int expected = (((Integer.hashCode(1000) * 31) * 31 + Float.hashCode(score)) * 31) + logs.hashCode(); eq(a.hashCode(), expected);
            }
            RichRecord nan = new RichRecord(null, null, Float.NaN, logs), otherNan = new RichRecord(null, null, Float.intBitsToFloat(0x7f800001), new ArrayList<>(logs));
            yes(nan.equals(otherNan)); eq(nan.hashCode(), Float.hashCode(Float.NaN) * 31 + logs.hashCode());
            eq(nan.hashCode(), otherNan.hashCode());
            yes(!new RichRecord(null, null, -0.0f, logs).equals(new RichRecord(null, null, 0.0f, logs)));
            PlaybackState idle = PlaybackState.Companion.getIDLE(); eq(idle.getBookId(), -1); eq(idle.getSpeed(), 1.0f); eq(idle.getLogs().size(), 0); eq(idle.getError(), null);
            SongPlaybackState song = SongPlaybackState.Companion.getIDLE(); eq(song.getPositionMs(), -1L); eq(song.getError(), null);
        });
        observe("data_body_members", () -> {
            RichRecord value = new RichRecord(null, null, 1.0f, Collections.emptyList()); eq(value.getActive(), false); eq(value.summary(), "missing:0");
            eq(RichRecord.class.getDeclaredMethods().length > 8, true); eq(PlaybackState.Companion.getIDLE().isActive(), false);
        });
    }
}
