# Task Scheduler Internals & Event Subscriptions

## Windows Event Log Channel
`earplugger` subscribes to:
- **Channel**: `Microsoft-Windows-Audio/Operational`
- **Provider**: `Microsoft-Windows-Audio`
- **Event ID**: `65`

> [!NOTE]
> On default installations of Windows 10 and Windows 11, this operational event channel is disabled. The `earplugger install` command automatically activates it using `%SystemRoot%\System32\wevtutil.exe sl Microsoft-Windows-Audio/Operational /e:true`.

## Event Subscription XPath Structure

```xml
<QueryList>
  <Query Id="0" Path="Microsoft-Windows-Audio/Operational">
    <Select Path="Microsoft-Windows-Audio/Operational">
      *[System[Provider[@Name='Microsoft-Windows-Audio'] and (EventID=65)]]
      and *[EventData[Data[@Name='DeviceName']='RODE NT-USB' and (Data[@Name='flow']='0' or Data[@Name='flow']='1') and Data[@Name='NewState']='1']]
    </Select>
  </Query>
</QueryList>
```

### XPath Literal Sanitization
Device names can contain single quotes or apostrophes (for example: `User's AirPods`). Because the Windows Event Log query engine (`wevtapi.dll`) implements a restricted subset of XPath 1.0 where functions like `concat()` are unsupported (producing error `15008`), `earplugger` formats strings containing apostrophes using double-quoted string literals:
`Data[@Name='DeviceName']="User's AirPods"` (which is XML-escaped to `Data[@Name=&apos;DeviceName&apos;]=&quot;User&apos;s AirPods&quot;`).

Strings containing both single and double quotes are rejected during CLI validation because `wevtapi.dll` does not support escaping quotes or `concat()` within event subscription queries.

Control characters (`< 0x20`) are stripped to preserve XML parser validity.

## Task Settings & Queuing
- **MultipleInstancesPolicy**: `Queue`
  Queues follow-up reconnect signals if multiple consecutive endpoint reconnect signals fire (e.g. capture endpoint arriving followed by master DAC clock 150ms later). This ensures subsequent device arrivals during the post-restart hold trigger a resynchronization rather than being dropped.
- **ExecutionTimeLimit**: Dynamically scaled based on `--delay-ms` (`PT30S` minimum for default 150ms delay, scaling up to `PT60S` for 30s delays to allow the full settle delay and Voicemeeter restart sequence to complete before termination).
- **DisallowStartIfOnBatteries**: `false` (operates normally on laptops).
- **RunLevel**: `LeastPrivilege` (runs under user logon context without elevated tokens).
