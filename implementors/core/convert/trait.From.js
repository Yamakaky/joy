(function() {var implementors = {};
implementors["dualshock_sys"] = [{"text":"impl From&lt;u8&gt; for Dpad","synthetic":false,"types":[]},{"text":"impl&lt;Id:&nbsp;ToPrimitive&gt; From&lt;Id&gt; for RawId&lt;Id&gt;","synthetic":false,"types":[]},{"text":"impl From&lt;i16&gt; for I16LE","synthetic":false,"types":[]},{"text":"impl From&lt;I16LE&gt; for i16","synthetic":false,"types":[]}];
implementors["gyromouse"] = [{"text":"impl From&lt;JoyKey&gt; for MapKey","synthetic":false,"types":[]},{"text":"impl From&lt;VirtualKey&gt; for MapKey","synthetic":false,"types":[]},{"text":"impl From&lt;(ActionType, ClickType)&gt; for ExtAction","synthetic":false,"types":[]}];
implementors["hid_gamepad"] = [{"text":"impl From&lt;HidError&gt; for GamepadError","synthetic":false,"types":[]},{"text":"impl From&lt;Error&gt; for GamepadError","synthetic":false,"types":[]}];
implementors["hid_gamepad_sys"] = [{"text":"impl From&lt;bool&gt; for KeyStatus","synthetic":false,"types":[]}];
implementors["joycon"] = [{"text":"impl From&lt;Report&gt; for Report","synthetic":false,"types":[]}];
implementors["joycon_sys"] = [{"text":"impl From&lt;u16&gt; for U16LE","synthetic":false,"types":[]},{"text":"impl From&lt;U16LE&gt; for u16","synthetic":false,"types":[]},{"text":"impl From&lt;i16&gt; for I16LE","synthetic":false,"types":[]},{"text":"impl From&lt;I16LE&gt; for i16","synthetic":false,"types":[]},{"text":"impl From&lt;u32&gt; for U32LE","synthetic":false,"types":[]},{"text":"impl From&lt;U32LE&gt; for u32","synthetic":false,"types":[]},{"text":"impl&lt;Id:&nbsp;ToPrimitive&gt; From&lt;Id&gt; for RawId&lt;Id&gt;","synthetic":false,"types":[]},{"text":"impl From&lt;bool&gt; for Bool","synthetic":false,"types":[]},{"text":"impl From&lt;u8&gt; for DeviceType","synthetic":false,"types":[]},{"text":"impl From&lt;u8&gt; for BatteryLevel","synthetic":false,"types":[]},{"text":"impl From&lt;InputReportEnum&gt; for InputReport","synthetic":false,"types":[]},{"text":"impl From&lt;SubcommandReplyEnum&gt; for SubcommandReply","synthetic":false,"types":[]},{"text":"impl From&lt;bool&gt; for PlayerLight","synthetic":false,"types":[]},{"text":"impl From&lt;IRRequestEnum&gt; for IRRequest","synthetic":false,"types":[]},{"text":"impl From&lt;IRAckRequestPacket&gt; for IRRequest","synthetic":false,"types":[]},{"text":"impl From&lt;IRReadRegisters&gt; for IRRequest","synthetic":false,"types":[]},{"text":"impl From&lt;MCUReportEnum&gt; for MCUReport","synthetic":false,"types":[]},{"text":"impl From&lt;MCURequestEnum&gt; for MCURequest","synthetic":false,"types":[]},{"text":"impl From&lt;IRRequest&gt; for MCURequest","synthetic":false,"types":[]},{"text":"impl From&lt;IRRequestEnum&gt; for MCURequest","synthetic":false,"types":[]},{"text":"impl From&lt;OutputReportEnum&gt; for OutputReport","synthetic":false,"types":[]},{"text":"impl From&lt;SubcommandRequest&gt; for OutputReport","synthetic":false,"types":[]},{"text":"impl From&lt;SubcommandRequestEnum&gt; for OutputReport","synthetic":false,"types":[]},{"text":"impl From&lt;MCURequest&gt; for OutputReport","synthetic":false,"types":[]},{"text":"impl From&lt;SubcommandRequestEnum&gt; for SubcommandRequest","synthetic":false,"types":[]},{"text":"impl From&lt;MCUCommand&gt; for SubcommandRequest","synthetic":false,"types":[]},{"text":"impl From&lt;AccessoryCommand&gt; for SubcommandRequest","synthetic":false,"types":[]},{"text":"impl From&lt;Sensitivity&gt; for SubcommandRequest","synthetic":false,"types":[]},{"text":"impl From&lt;SPIReadRequest&gt; for SubcommandRequest","synthetic":false,"types":[]},{"text":"impl From&lt;SPIWriteRequest&gt; for SubcommandRequest","synthetic":false,"types":[]},{"text":"impl From&lt;PlayerLights&gt; for SubcommandRequest","synthetic":false,"types":[]},{"text":"impl From&lt;HomeLight&gt; for SubcommandRequest","synthetic":false,"types":[]},{"text":"impl From&lt;ControllerColor&gt; for SPIWriteRequest","synthetic":false,"types":[]},{"text":"impl From&lt;UseSPIColors&gt; for SPIWriteRequest","synthetic":false,"types":[]},{"text":"impl From&lt;SensorCalibration&gt; for UserSensorCalibration","synthetic":false,"types":[]},{"text":"impl From&lt;UserSensorCalibration&gt; for SPIWriteRequest","synthetic":false,"types":[]}];
if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()