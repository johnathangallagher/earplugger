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
Device names can contain single quotes or apostrophes (for example: `User's AirPods`). Because XPath 1.0 does not support character escaping within single quotes, `earplugger` formats strings containing apostrophes using the XPath `concat()` function:
`concat('User', "'", 's AirPods')`

Control characters (`< 0x20`) are stripped to preserve XML parser validity.

## Task Settings & Queuing
- **MultipleInstancesPolicy**: `IgnoreNew`
  Ensures that if multiple consecutive endpoint reconnect signals fire (e.g. render and capture endpoints registering simultaneously upon KVM toggle), the in-flight run handles the restart while secondary triggers are dropped, eliminating restart storms.
- **ExecutionTimeLimit**: `PT1M` (1 minute limit to prevent runaway tasks).
- **DisallowStartIfOnBatteries**: `false` (operates normally on laptops).
- **RunLevel**: `LeastPrivilege` (runs under user logon context without elevated tokens).
