"""Pure serialization helpers for the Unity scene evidence compiler."""

import struct


def vector2(value):
    return {"x": float(value.x), "y": float(value.y)} if value is not None else None


def vector3(value):
    return {"x": float(value.x), "y": float(value.y), "z": float(value.z)} if value is not None else None


def quaternion(value):
    return {"x": float(value.x), "y": float(value.y), "z": float(value.z), "w": float(value.w)} if value is not None else None


def color(value):
    return {"r": float(value.r), "g": float(value.g), "b": float(value.b), "a": float(value.a)} if value is not None else None


def mono_behaviour_header(payload):
    if len(payload) < 32:
        raise ValueError(f"MonoBehaviour payload is shorter than its 32-byte header: {len(payload)}")
    game_file_id, game_path_id = struct.unpack_from("<iq", payload, 0)
    script_file_id, script_path_id = struct.unpack_from("<iq", payload, 16)
    return {"gameObjectFileId": game_file_id, "gameObjectPathId": game_path_id, "enabled": bool(payload[12]),
            "scriptFileId": script_file_id, "scriptPathId": script_path_id}
