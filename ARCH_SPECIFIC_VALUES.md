# Architecture-Specific Values in OSDI Snapshots

## Problem

Multiple values in the OSDI descriptor snapshots are architecture-dependent because they represent memory offsets and sizes calculated by LLVM using platform-specific ABI rules.

## Architecture-Dependent Values

### 1. `instance size` ✓ CONFIRMED
**Line**: `instance size 1864`

**Source**: `openvaf/osdi/src/metadata.rs:376`
```rust
let instance_size =
    LLVMABISizeOfType(*target_data, NonNull::from(inst_data.ty).as_ptr()) as u32;
```

**Why**: LLVM calculates struct size including padding based on platform ABI alignment rules.

**Example differences**:
- BSIM3: 4512 (x86-64) vs 4568 (ARM64) = +56 bytes
- amplifier: 224 (x86-64) vs 240 (ARM64) = +16 bytes

---

### 2. `residual` offset values ✓ CONFIRMED
**Line**: `residual 256 264 4294967295 4294967295`

The four numbers are:
1. resist_residual_off
2. react_residual_off
3. resist_limit_rhs_off
4. react_limit_rhs_off

**Source**: `openvaf/osdi/src/inst_data.rs:472-489`
```rust
pub fn residual_off(..., target_data: &LLVMTargetDataRef) -> Option<u32> {
    let off = unsafe {
        LLVMOffsetOfElement(*target_data, NonNull::from(self.ty).as_ptr(), elem)
    } as u32;
    Some(off)
}
```

**Why**: Offsets into the instance data struct depend on field alignment.

**Example differences** (diode.va):
- Node A resist: 248 (old) → 256 (new)
- Node A react: 256 (old) → 264 (new)

---

### 3. `react_ptr` values ✓ CONFIRMED
**Line**: `jacobian (A, A) JacobianFlags(...) react_ptr = 120`

**Source**: `openvaf/osdi/src/metadata.rs:265-267`
```rust
let mut jacobian_ptr_react_offset = unsafe {
    LLVMOffsetOfElement(*target_data, NonNull::from(inst_data.ty).as_ptr(), JACOBIAN_PTR_REACT)
} as u32;
```

**Why**: Offset to the reactive jacobian pointer array in the instance struct.

**Note**: Often `4294967295` (u32::MAX) meaning "not present", but when present, the value is arch-specific.

---

### 4. `model size` ⚠️ TO VERIFY
**Line**: `model size 120`

**Source**: `openvaf/osdi/src/metadata.rs:378`
```rust
let model_size =
    LLVMABISizeOfType(*target_data, NonNull::from(model_data.ty).as_ptr()) as u32;
```

**Status**: Similar to instance_size, this is likely arch-specific. Need to verify if it changes.

---

## Non-Architecture-Specific Values

These values **should be** the same across platforms:

- Parameter names, types, descriptions
- Node names, units, is_flow flags
- Number of terminals, parameters, states
- Jacobian flags (which entries are const, have react, etc.)
- Nature and discipline information
- runits strings (units of flow/potential)

## Solutions

### Option 1: Remove All Arch-Specific Values from Snapshots ✅ RECOMMENDED

**Implementation**:
1. Comment out or skip these in Debug formatter:
   - `instance size`
   - `model size`
   - `residual` offset numbers (but keep the field itself as marker)
   - `react_ptr` values (but keep the field)

2. Replace with markers:
   ```
   node "A" units = "V", runits = "A"
   residual <arch-specific offsets>
   jacobian (A, A) JacobianFlags(...) react_ptr = <arch-specific>
   instance size <arch-specific>
   model size <arch-specific>
   ```

**Pros**:
- Clean, platform-agnostic snapshots
- Still test the important structural information
- Easy to maintain

**Cons**:
- Lose some regression detection for layout changes
- Can't detect if offsets become corrupted

---

### Option 2: Test Field Count/Types Instead of Offsets

Instead of testing exact offsets, verify:
- Number of fields is correct
- Field types are correct
- Field order is correct

This would require a different test approach entirely.

---

### Option 3: Normalize to Field Index

Instead of printing byte offsets, print field indices:
```
residual field=12 field=13 field=MAX field=MAX
```

This would be arch-independent but still verify structure.

**Pros**:
- Still verifies layout without being arch-specific
- Detects field ordering changes

**Cons**:
- Requires code changes to track field indices
- Less directly readable

---

## Recommended Action

**Go with Option 1**: Remove arch-specific numeric values from snapshots.

Modified output would look like:
```
node "A" units = "V", runits = "A"
residual
jacobian (A, A) JacobianFlags(JACOBIAN_ENTRY_RESIST | JACOBIAN_ENTRY_REACT)
...
0 states
has bound_step false
```

We remove:
- The four residual offset numbers
- The `react_ptr = XXX` values
- The `instance size XXX` line
- The `model size XXX` line

This preserves all the semantically important information while being platform-independent.
